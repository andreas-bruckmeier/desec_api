# deSEC Client

Unofficial client library for the [deSEC](https://desec.io/) DNS API.

deSEC is a free DNS hosting service, designed with security in mind.
Running on open-source software and supported by [SSE](https://securesystems.de/), deSEC is free for everyone to use.

## Supported API endpoints

# Supported endpoints

* Manage accounts
  * Obtain a Captcha
  * Register Account with optional domain creation
  * Log In (Retrieve API token using email & password)
  * Log Out (When client was created from credentials)
  * Retrieve account information
  * Modify account settings (only updating outreach\_preference is supported by the API)
  * Password reset (Request for password reset & confirmation, but handling of approval via mail needs to be handled)
  * Password change
  * Change of email address
  * Delete account

* Manage domains
  * Creating a domain
  * List domains
  * Retrieve a specific domain
  * Identifying the responsible domain for a DNS name
  * Exporting a domain as zonefile
  * Deleting a domain

* Manage DNS records
  * Creating an RRset
  * Retrieving all RRsets in a Zone
  * Retrieving all RRsets in a Zone filtered by type
  * Retrieving all RRsets in a Zone filtered by subname
  * Retrieving a Specific RRset
  * Modifying an RRset
  * Deleting an RRset

* Manage Tokens
  * Create a token
  * Modify a token
  * List all tokens
  * Retrieve a specific token
  * Delete a token

* Manage Token Policies
  * Create a token policy (including default policy)
  * Modify a token policy
  * List all token policies
  * Delete a token policy

## Currently not supported

* Pagination when over 500 items exist
* Manage DNS records
  * Bulk operations when modifying or deleting RRsets

## License

See [LICENSE-MIT](LICENSE-MIT) for details.

## Testing

Integrations tests depend on setting the following environment variables

* `DESEC_TOKEN`
* `DESEC_DOMAIN`
* `DESEC_EMAIL`
* `DESEC_PASSWORD`
* `DESEC_ACCOUNT_INFO`
* `DESEC_TOKEN_ID`

You will need to create a [desec](https://desec.io) account (recommended that you do not use an
account used in production!). Create a domain in that account, `domain.test` for example, and then
create an API token within that domain. Make sure to enable `Can manage tokens` under the advanced
settings for the new token.

You can then retrieve the token ID and account info from the API, for example using any CLI client.
Note that account info is a JSON string.
