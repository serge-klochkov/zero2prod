## Zero To Production In Rust

### Pre-commit hooks

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
