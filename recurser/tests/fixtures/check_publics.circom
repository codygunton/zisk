// Example CheckPublics — no-op (no stitching constraints).
//
// Required signature: must declare `template CheckPublics(nPublics, nPrivateInputs)`
// with `signal input a_publics[nPublics]`, `signal input b_publics[nPublics]`, and
// `signal input private_inputs[nPrivateInputs]`. The template emits `===`
// constraints; a failed constraint aborts the fold.
template CheckPublics(nPublics, nPrivateInputs) {
    signal input a_publics[nPublics];
    signal input b_publics[nPublics];
    signal input private_inputs[nPrivateInputs];

    // Drain unused inputs so Circom doesn't complain.
    for (var i = 0; i < nPublics; i++) {
        _ <== a_publics[i];
        _ <== b_publics[i];
    }
    for (var i = 0; i < nPrivateInputs; i++) {
        _ <== private_inputs[i];
    }
}
