# Source-owned pure function compiler witnesses

`range-assembly-baseline.edict` is copied byte-for-byte from Jim (GitHub
`flyingrobots/jedit`) commit `327ac11c71cc5a9a33c5552be7a6db533bf88f5d`,
`edict/replace-range-probes/range-assembly/RangeAssembly.edict`.
Its SHA-256 is `8e8d2703759fb30cb49b73650ba8ddfd637d68b33b86145661bfca3033c968fc`.

`range-assembly.edict` factors that same expression into a called two-argument
source function with an immutable local binding. The existing input/output
bounds, application import and intent contract are preserved. Both are Edict
source specimens; no handwritten Core or Target artifact substitutes for them.

The source-function tests use explicit profile/budget context facts for isolated
compiler checks. The CLI test invokes the public `project` operation. Neither
witness is application-build, Echo runtime, retained-buffer decoding, or rope
mutation evidence. Those require the separate public application and consumer
witnesses in Edict226, Echo752 and Jim296.
