# Zero To Production In Rust

## Cargo config

See the comments in `.cargo/config.yml` and install the required tools for your platform

## Pre-commit hooks

You will need to install `pre-commit` tool and generate the Git hooks:

```
pip install pre-commit
pre-commit install
```

Then, you should be able to verify the installation:

```
pre-commit run --all-files 
```

the output should look like this:

```
[INFO] Initializing environment for https://github.com/doublify/pre-commit-rust.
fmt......................................................................Passed
clippy...................................................................Passed
cargo check..............................................................Passed
```

## API

### POST /api/subscriptions

#### Description

Create a pending subscription to the newsletter. 
If background email sending task fails, the subscription will be marked as failed.

#### Headers

Content-Type: application/x-www-form-urlencoded
 
#### Request (form data)

```
email: <non-empty string, valid email>
name: <non-empty string>
```

#### Responses

* 200 OK - saved a new subscription or re-send pending or failed subscription confirmation
* 409 Conflict - specified email already exists as a confirmed subscription
* 500 ISE - unexpected error

---

### GET /api/subscriptions/confirm?subscription_token=UUID

#### Description

Confirm a pending subscription to the newsletter. 
Subcription token should be a valid UUID string.

#### Responses

* 200 OK - subscription confirmed
* 401 Unauthorized - token not found
* 500 ISE - unexpected error
---

## Differences from the suggested implementation in the book

* YAML-based configs replaced with dotenv style config reader 
(OS env first, then `.env.local` and `.env` files). See `./config.rs`
* Instead of synchronously sending an email on new subscription creation, 
NATS is used as a message broker, enabling background email sending 
as it is a third party dependency and should not block the main path.
* `eventually` helper in `test/common.rs` module. 
Helps waiting only required amount of time until an async background operation 
(such as NATS event handling) is completed.
* DB and handlers layers are separated from the `routes` module
* More complex test subscriptions flows and generally more coverage