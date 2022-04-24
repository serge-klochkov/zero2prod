# Zero To Production In Rust

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

Content-Type: application/x-www-form-urlencoded
 
Request (form data)

```
email: <non-empty string, valid email>
name: <non-empty string>
```

Responses

* 200 OK, empty body
* 500 ISE, empty body (temporary)


## Differences from the suggested implementation in the book

* YAML-based configs replaced with dotenv style config reader 
(OS env first, then `.env.local` and `.env` files). See `./config.rs`
* Instead of synchronously sending an email on new subscription creation, 
NATS is used as a message broker, enabling background email sending 
as it is a third party dependency and should not block the main path.
* DB layer is separated from `routes` module