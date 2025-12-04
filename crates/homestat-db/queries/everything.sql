SELECT
    recv_timestamp, source, micros,
    reading.celcius_whole, reading.celcius_tenth,
    reading.humidity_whole, reading.humidity_tenth,
    error.error
FROM receive
LEFT JOIN reading ON receive.reading = reading.id
LEFT JOIN error ON receive.error = error.id
LIMIT ?
OFFSET ?;
