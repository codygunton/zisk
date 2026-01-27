//! Witness capture for oracle reads during emulation.
//!
//! This module provides utilities to capture oracle reads during Zisk emulation,
//! producing a witness file that can be replayed later.
//--say more about why

use std::sync::{Arc, Mutex};
use zisk_core::{OracleCallback, OracleOp, ZiskMemoryReader};

/// Captures oracle reads into a Vec<u32> while forwarding to an underlying oracle.
///
/// During witness generation, all oracle reads are recorded. This captured data
/// can later be serialized to a witness file for replay execution.
pub struct WitnessCapture<F>
where
    F: FnMut(OracleOp, &dyn ZiskMemoryReader) -> u64 + Send + 'static,
{
    inner: F,
    reads: Arc<Mutex<Vec<u32>>>,
}

impl<F> WitnessCapture<F>
where
    F: FnMut(OracleOp, &dyn ZiskMemoryReader) -> u64 + Send + 'static,
{
    /// Create a new witness capture wrapping an inner oracle callback.
    pub fn new(inner: F) -> Self {
        Self { inner, reads: Arc::new(Mutex::new(Vec::new())) }
    }

    /// Get a reference to the captured reads vector.
    pub fn get_reads(&self) -> Arc<Mutex<Vec<u32>>> {
        Arc::clone(&self.reads)
    }

    /// Convert this into an OracleCallback.
    ///
    /// Consumes self and returns a callback that can be used with the emulator.
    pub fn into_callback(self) -> OracleCallback {
        let inner = Arc::new(Mutex::new(self.inner));
        let reads = self.reads;

        Arc::new(Mutex::new(move |op: OracleOp, mem_reader: &dyn ZiskMemoryReader| -> u64 {
            let mut inner_guard = inner.lock().expect("inner oracle lock poisoned");
            match op {
                OracleOp::Read => {
                    let value = inner_guard(OracleOp::Read, mem_reader);
                    // Capture the read value as u32
                    let value_u32 = value as u32;
                    reads.lock().expect("reads lock poisoned").push(value_u32);
                    value
                }
                OracleOp::Write(val) => inner_guard(OracleOp::Write(val), mem_reader),
            }
        }))
    }
}

/// Create a witness-capturing oracle callback from separate read and write functions.
///
/// This is useful when bridging to an oracle that has separate read/write methods
/// (like `ZkEENonDeterminismSource`).
///
/// Returns a tuple of:
/// - The `OracleCallback` to use with the emulator
/// - A reference to the captured reads vector
///
/// # Example
///
/// ```ignore
/// use ziskemu::witness::create_witness_capture_callback;
///
/// let (callback, reads_ref) = create_witness_capture_callback(
///     || oracle.read(),
///     |v| oracle.write(v),
/// );
///
/// // Run emulator with callback...
///
/// // Get the captured witness
/// let witness = reads_ref.lock().unwrap().clone();
/// ```
pub fn create_witness_capture_callback<R, W>(
    read_fn: R,
    write_fn: W,
) -> (OracleCallback, Arc<Mutex<Vec<u32>>>)
where
    R: FnMut() -> u32 + Send + 'static,
    W: FnMut(u32) + Send + 'static,
{
    let read_fn = Arc::new(Mutex::new(read_fn));
    let write_fn = Arc::new(Mutex::new(write_fn));
    let reads: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
    let reads_clone = Arc::clone(&reads);

    let callback: OracleCallback =
        Arc::new(Mutex::new(move |op: OracleOp, _mem_reader: &dyn ZiskMemoryReader| -> u64 {
            match op {
                OracleOp::Read => {
                    let value = {
                        let mut rf = read_fn.lock().expect("read_fn lock poisoned");
                        rf()
                    };
                    reads.lock().expect("reads lock poisoned").push(value);
                    value as u64
                }
                OracleOp::Write(val) => {
                    let mut wf = write_fn.lock().expect("write_fn lock poisoned");
                    wf(val as u32);
                    0
                }
            }
        }));

    (callback, reads_clone)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_core::ZiskMemoryReader;

    /// Dummy memory reader for tests - panics if actually used
    struct DummyMemReader;

    impl ZiskMemoryReader for DummyMemReader {
        fn read_mem(&self, addr: u64, width: u64) -> u64 {
            panic!("DummyMemReader::read_mem() called with addr={addr:x} width={width}");
        }
    }

    #[test]
    fn test_witness_capture_records_reads() {
        // Create a simple oracle that returns incrementing values
        let counter = Arc::new(Mutex::new(0u32));
        let counter_clone = Arc::clone(&counter);

        let inner = move |op: OracleOp, _mem_reader: &dyn ZiskMemoryReader| -> u64 {
            match op {
                OracleOp::Read => {
                    let mut c = counter_clone.lock().unwrap();
                    let val = *c;
                    *c += 1;
                    val as u64
                }
                OracleOp::Write(_) => 0,
            }
        };

        let capture = WitnessCapture::new(inner);
        let reads_ref = capture.get_reads();
        let callback = capture.into_callback();
        let dummy_mem = DummyMemReader;

        // Perform some reads
        {
            let mut cb = callback.lock().unwrap();
            assert_eq!(cb(OracleOp::Read, &dummy_mem), 0);
            assert_eq!(cb(OracleOp::Read, &dummy_mem), 1);
            assert_eq!(cb(OracleOp::Read, &dummy_mem), 2);
        }

        // Check captured reads
        let reads = reads_ref.lock().unwrap();
        assert_eq!(*reads, vec![0, 1, 2]);
    }

    #[test]
    fn test_create_witness_capture_callback() {
        let counter = Arc::new(Mutex::new(10u32));
        let counter_clone = Arc::clone(&counter);
        let writes = Arc::new(Mutex::new(Vec::new()));
        let writes_clone = Arc::clone(&writes);

        let (callback, reads_ref) = create_witness_capture_callback(
            move || {
                let mut c = counter_clone.lock().unwrap();
                let val = *c;
                *c += 1;
                val
            },
            move |v| {
                writes_clone.lock().unwrap().push(v);
            },
        );

        let dummy_mem = DummyMemReader;

        // Perform reads and writes
        {
            let mut cb = callback.lock().unwrap();
            assert_eq!(cb(OracleOp::Read, &dummy_mem), 10);
            cb(OracleOp::Write(100), &dummy_mem);
            assert_eq!(cb(OracleOp::Read, &dummy_mem), 11);
            cb(OracleOp::Write(200), &dummy_mem);
        }

        // Check captured reads
        let reads = reads_ref.lock().unwrap();
        assert_eq!(*reads, vec![10, 11]);

        // Check writes were forwarded
        let w = writes.lock().unwrap();
        assert_eq!(*w, vec![100, 200]);
    }
}
