SELECT id
FROM reading
WHERE (
    celcius_whole = ?
    AND celcius_tenth = ?
    AND humidity_whole = ?
    AND humidity_tenth = ?
);
