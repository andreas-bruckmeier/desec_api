# deSEC Client

Unofficial client library for the (deSEC)[https://desec.io/] DNS API.

deSEC is a **free DNS hosting** service, **designed with security in mind**.
Running on **open-source software** and **supported by [SSE](https://securesystems.de/)**, deSEC is free for everyone to use.

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
