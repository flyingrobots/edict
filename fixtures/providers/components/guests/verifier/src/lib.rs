wit_bindgen::generate!({
    path: "../../../../../docs/abi/edict-target-provider.wit",
    world: "verifier",
});

use edict::target_provider::protocol::{
    Artifact, VerificationOutputArtifact, VerificationSuccessV1,
};

struct Fixture;

impl Guest for Fixture {
    fn verify(request: VerificationRequestV1) -> VerificationResultV1 {
        Ok(VerificationSuccessV1 {
            outputs: request
                .requested_outputs
                .into_iter()
                .map(|output| VerificationOutputArtifact {
                    role: output.role,
                    kind: output.kind,
                    artifact: Artifact {
                        domain: output.domain,
                        bytes: vec![0xf6],
                    },
                    logical_path: None,
                })
                .collect(),
            diagnostics: Vec::new(),
        })
    }
}

export!(Fixture);
