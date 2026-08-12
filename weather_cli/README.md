# Weather CLI Application

source: https://www.youtube.com/watch?v=kV4a_fOGVQg

## Dependencies
- **reqwest** - HTTP client for making API requests
- **serde** - Framework for serializing and deserializing Rust data structures
- **serde_json** - JSON support for serde serialization/deserialization
- **colored** - Library for adding colors to terminal text output

```
cargo add reqwest --features json,blocking
cargo add serde --features derive
cargo add serde_json
cargo add colored
```