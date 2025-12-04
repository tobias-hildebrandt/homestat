PRAGMA foreign_keys = ON;
PRAGMA journal_mode = DELETE;

-- Server receives
CREATE TABLE receive (
    -- ID
    id INTEGER NOT NULL PRIMARY KEY,
    -- UNIX millisecond timestamp of server reception
    recv_timestamp INTEGER NOT NULL,
    -- Source Pico identifier
    source INTEGER NOT NULL,
    -- Pico clock micros
    micros INTEGER NOT NULL,

    -- ID of reading (nullable)
    reading INTEGER REFERENCES reading(id),
    -- ID of error (nullable)
    error INTEGER REFERENCES error(id),

    -- must have reading XOR error
    CONSTRAINT valid_data CHECK (
        (reading IS NOT NULL AND error IS NULL)
        OR
        (reading IS NULL AND error IS NOT NULL)
    )
) STRICT;

-- Timestamp index
CREATE INDEX timestamp ON receive(
    recv_timestamp
);

-- Readings
-- INSERTs must not use last_rowid
CREATE TABLE reading (
    -- ID
    id INTEGER NOT NULL PRIMARY KEY,
    -- Whole degree Celcius
    celcius_whole INTEGER NOT NULL,
    -- Tenths degree Celcius
    celcius_tenth INTEGER NOT NULL,
    -- Whole percent relative humidity
    humidity_whole INTEGER NOT NULL,
    -- Tenth percent relative humidity
    humidity_tenth INTEGER NOT NULL,

    -- never store duplicate data
    CONSTRAINT unique_readings UNIQUE (
        celcius_whole,
        celcius_tenth,
        humidity_whole,
        humidity_tenth
    ) ON CONFLICT IGNORE
 ) STRICT;

-- Errors
-- INSERTs must not use last_rowid
CREATE TABLE error (
    -- ID
    id INTEGER NOT NULL PRIMARY KEY,
    -- Serialized error
    error TEXT NOT NULL,

    -- never store duplicate data
    CONSTRAINT unique_errors UNIQUE (
        error
    ) ON CONFLICT IGNORE
 ) STRICT;
