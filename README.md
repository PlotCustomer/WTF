# Store App

The **Store App Setup** work package: a franchisor-facing application for registering
franchisees and opening their stores. The app is the deployable Store App component
from the model and realizes the Store operations service; `Franchisee` and `Store`
are persisted as PostgreSQL data objects.

## What it does

- Register a franchisee with a name and email address.
- Open a store for a franchisee with a name and address.
- See the setup workspace: franchisee and store counts, franchisee directories,
  and each store's owner.
- JSON endpoints expose the same store-operations data for the app and its consumers.

The UI uses a server-rendered setup workspace. Every write is a POST/redirect/GET
flow with validation, and all data is stored in the Plot-managed PostgreSQL service.
There is no demo or seed data: the first franchisee starts an empty workspace.

## HTTP surface

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Setup workspace and directory |
| `POST` | `/franchisees` | Register a franchisee (HTML form) |
| `POST` | `/stores` | Open a store (HTML form) |
| `GET` | `/api/franchisees` | List franchisees with store counts |
| `POST` | `/api/franchisees` | Register a franchisee (`name`, `email`) |
| `GET` | `/api/stores` | List stores with their franchisee |
| `POST` | `/api/stores` | Open a store (`franchisee_id`, `name`, `address`) |
| `GET` | `/health` | Liveness |
| `GET` | `/ready` | Readiness, including the database |
| `GET` | `/static/app.css` | Embedded stylesheet |

## Running it

The delivery contract is `.plot/plot.toml`. The toolchain is Rust 1.99.1, and the
server is served over HTTPS on port 3000. The server binds `0.0.0.0` by default and
reads its settings from the environment:

| Variable | Meaning | Default |
| --- | --- | --- |
| `DATABASE_URL` | PostgreSQL connection URL | required |
| `HOST` | Bind address | `0.0.0.0` |
| `PORT` | Listen port | `3000` |
| `TLS_CERT_PATH` | PEM certificate path | `/tmp/store-app-tls/cert.pem` |
| `TLS_KEY_PATH` | PEM private key path | `/tmp/store-app-tls/key.pem` |
| `TLS_SUBJECT_ALT_NAMES` | Extra comma-separated certificate names | none |
| `RUST_LOG` | Log filter | `store_app=info,tower_http=info` |

A self-signed certificate is generated on first start when the configured certificate
files do not exist. Database migrations run automatically at startup.

```sh
cargo fetch
cargo build --release
DATABASE_URL=postgres://… scripts/start.sh
```

## Development checks

```sh
cargo fmt --all -- --check
cargo check --all-targets
cargo test
cargo clippy --all-targets -- -D warnings
```

The tests cover franchisee and store validation, the setup template, the embedded
stylesheet, liveness, and the HTML not-found page.

## Model mapping

| Model element | Realisation |
| --- | --- |
| `Store App Setup` (WorkPackage) | This delivery |
| `Store App` (ApplicationComponent) | The Rust service in `src/` |
| `Store operations service` (ApplicationService) | The `/api/franchisees` and `/api/stores` operations, plus the setup workspace |
| `Franchisee`, `Store` (DataObject) | PostgreSQL tables in `migrations/0001_store_setup.sql` |
| `Franchisee` (BusinessActor) | The person or company using the setup workspace to open stores |

The capability and business actors above the assignment are the reason the work
exists and are not separate software components. The repository started empty, so
there were no existing repository conventions or conflicting model names to reconcile.
