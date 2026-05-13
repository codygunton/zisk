// Example PreparePublics — identity passthrough.
//
// Required signature: must declare `template PreparePublics(nPublics, nPrivateInputs)`
// with `signal input publics[nPublics]`, `signal input private_inputs[nPrivateInputs]`,
// `signal output recurser_publics[nPublics]`.
template PreparePublics(nPublics, nPrivateInputs) {
    signal input publics[nPublics];
    signal input private_inputs[nPrivateInputs];
    signal output recurser_publics[nPublics];

    // Drain unused private inputs.
    for (var i = 0; i < nPrivateInputs; i++) {
        _ <== private_inputs[i];
    }

    for (var i = 0; i < nPublics; i++) {
        recurser_publics[i] <== publics[i];
    }
}
