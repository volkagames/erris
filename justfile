set export
set dotenv-load

default:
  @just --list

lint:
    cargo deny --log-level error check advisories bans sources
    cargo fmt --all --check -- --unstable-features --error-on-unformatted
    cargo check
    cargo check -p erris --no-default-features
    cargo check -p erris --all-features
    cargo clippy
    cargo sort -c -w
    cargo machete

fix:
    cargo clippy --fix --allow-dirty --allow-staged --all-features --all-targets
    cargo fmt --all -- --unstable-features --error-on-unformatted
    cargo sort -w
    cargo machete --fix

publish:
    cargo set-version ${CI_COMMIT_TAG#[vV]}
    cargo check
    cargo publish --allow-dirty --locked --no-verify $@

test *args='':
    cargo nextest run --run-ignored default $args
