use std::collections::BTreeMap;

use serde::Deserialize;

wit_bindgen::generate!({
    path: "../../wit",
    world: "datahub-plugin",
});

struct EchoPlugin;

#[derive(Deserialize)]
struct Request {
    inputs: BTreeMap<String, Vec<u8>>,
}

impl Guest for EchoPlugin {
    fn run(input: Vec<u8>) -> Result<Vec<u8>, String> {
        let request: Request =
            serde_json::from_slice(&input).map_err(|error| error.to_string())?;
        let mode = request.inputs.values().next().map(Vec::as_slice);
        if mode == Some(b"spin") {
            loop {
                core::hint::spin_loop();
            }
        }
        if mode == Some(b"oversize") {
            return Ok(vec![b'x'; 2 * 1024 * 1024]);
        }
        if mode == Some(b"memory") {
            return Ok(vec![b'x'; 128 * 1024 * 1024]);
        }
        Ok(input)
    }
}

export!(EchoPlugin);
