SET time_zone = 'Asia/Shanghai';

-- Test show statement;

SHOW VARIABLES time_zone;

SHOW VARIABLES system_time_zone;

-- Test create table default value

-- 1. In UTC

SET time_zone = 'UTC';

CREATE TABLE table_default_value (
  ts timestamp(3) time index,
  test timestamp(3) DEFAULT '1970-01-01 08:00:00' PRIMARY KEY,
);

INSERT INTO table_default_value (ts) VALUES (0);

SELECT test::Int64 from table_default_value;

DROP TABLE table_default_value;

-- 2. In Asia/Shanghai

SET time_zone = 'Asia/Shanghai';

CREATE TABLE table_default_value (
  ts timestamp(3) time index,
  test timestamp(3) DEFAULT '1970-01-01 08:00:00' PRIMARY KEY,
);

INSERT INTO table_default_value (ts) VALUES (0);

SELECT test::Int64 from table_default_value;

DROP TABLE table_default_value;

-- Test Insert value

-- 1. In UTC

SET time_zone = 'UTC';

CREATE TABLE insert_value (
  ts timestamp(3) time index,
  host STRING PRIMARY KEY,
);

INSERT INTO TABLE insert_value VALUES
    ("1970-01-01 08:00:00", 'host1');

SELECT ts::Int64 from insert_value;

DELETE FROM insert_value;

-- 2. In Asia/Shanghai

SET time_zone = 'Asia/Shanghai';

INSERT INTO TABLE insert_value VALUES
    ("1970-01-01 08:00:00", 'host1');

SELECT ts::Int64 from insert_value;

DROP TABLE insert_value;

-- Test Range query

CREATE TABLE host (
  ts timestamp(3) time index,
  host STRING PRIMARY KEY,
  val BIGINT,
);

INSERT INTO TABLE host VALUES
    ("1970-01-01T22:30:00+00:00", 'host1', 0),
    ("1970-01-01T23:30:00+00:00", 'host1', 1),
    ("1970-01-02T22:30:00+00:00", 'host1', 2),
    ("1970-01-02T23:30:00+00:00", 'host1', 3),
    ("1970-01-01T22:30:00+00:00", 'host2', 4),
    ("1970-01-01T23:30:00+00:00", 'host2', 5),
    ("1970-01-02T22:30:00+00:00", 'host2', 6),
    ("1970-01-02T23:30:00+00:00", 'host2', 7);

SET time_zone = 'UTC';

SELECT ts::Int64 AS t, host, min(val) RANGE '1d' FROM host ALIGN '1d' TO '1900-01-01T00:00:00+01:00' ORDER BY host, t;

SET time_zone = '+01:00';

SELECT ts::Int64 AS t, host, min(val) RANGE '1d' FROM host ALIGN '1d' ORDER BY host, t;

DROP TABLE host;

SET time_zone = 'UTC';
