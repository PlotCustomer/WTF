CREATE TABLE franchisees (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL CHECK (char_length(btrim(name)) BETWEEN 1 AND 120),
    email TEXT NOT NULL CHECK (char_length(btrim(email)) BETWEEN 3 AND 254),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX franchisees_email_unique ON franchisees (lower(email));

CREATE TABLE stores (
    id UUID PRIMARY KEY,
    franchisee_id UUID NOT NULL REFERENCES franchisees (id) ON DELETE RESTRICT,
    name TEXT NOT NULL CHECK (char_length(btrim(name)) BETWEEN 1 AND 120),
    address TEXT NOT NULL CHECK (char_length(btrim(address)) BETWEEN 1 AND 200),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX stores_franchisee_id_idx ON stores (franchisee_id);

COMMENT ON TABLE franchisees IS 'A person or company operating one or more stores.';
COMMENT ON TABLE stores IS 'A physical store opened by a franchisee.';
