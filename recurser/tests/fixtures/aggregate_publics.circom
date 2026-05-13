// Example AggregatePublics — inherits every slot from A (the "prev" side).
//
// Required signature: must declare `template AggregatePublics(nPublics, nPrivateInputs)`
// with `signal output aggregated_publics[nPublics]`, `signal input a_publics[nPublics]`,
// `signal input b_publics[nPublics]`, `signal input private_inputs[nPrivateInputs]`.
// Every element of `aggregated_publics` must be driven by `<==`.
template AggregatePublics(nPublics, nPrivateInputs) {
    signal output aggregated_publics[nPublics];
    signal input a_publics[nPublics];
    signal input b_publics[nPublics];
    signal input private_inputs[nPrivateInputs];

    // Drain unused B-side inputs + private inputs.
    for (var i = 0; i < nPublics; i++) {
        _ <== b_publics[i];
    }
    for (var i = 0; i < nPrivateInputs; i++) {
        _ <== private_inputs[i];
    }

    for (var i = 0; i < nPublics; i++) {
        aggregated_publics[i] <== a_publics[i];
    }
}
