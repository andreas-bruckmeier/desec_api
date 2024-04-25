# deSEC Client

Client library for the deSEC Free Secure DNS service focussing on retrieving and creating DNS Records

## How to use in your project

Add dependency to your Cargo.toml

```toml
[package]
name = "my-project"
version = "0.1.0"
edition = "2021"

[dependencies]
desec-client = { git = "https://github.com/autopwn/desec-client", tag = "v0.2.0" }
jsonformat = "2.0.0"
```

Use it in your code

```rust
use desec_client::DeSecClient;

fn main() {
    let token = std::env::var("DESEC_API_TOKEN").unwrap();
    let client = DeSecClient::new(token);
    let account_info = &client.get_account_info().unwrap();
    println!("{:#?}", account_info);
}
```

Output

```rust
AccountInformation {
    created: "2020-08-25T11:27:08.362000Z",
    email: "info@example.com",
    id: "a5352169-c7bd-4d48-8818-bc90e7d7a1a8",
    limit_domains: 15,
    outreach_preference: true,
}
```

## License

See [LICENSE](LICENSE) for details.
