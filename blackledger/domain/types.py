from enum import IntEnum
from typing import Optional
from uuid import UUID

import base58
from pydantic import constr
from ulid import ULID

Base58String = constr(pattern=r"^[1-9A-HJ-NP-Za-km-z]+$")
CurrencyCode = constr(pattern=r"^[A-Z][A-Z0-9\.\-_]*[A-Z0-9]$")
NameString = constr(pattern=r"^[\w\-\. ]+$")


class NormalType(IntEnum):
    DR = 1
    CR = -1

    @classmethod
    def field_converter(cls, value):
        if isinstance(value, str):
            if value not in cls.__members__:
                raise ValueError("Not a valid NormalType value")
            value = cls.__members__[value]
        return value


class ID(UUID):
    """
    ID is generated from ULID, stored as a 128-bit UUID internally, and represented as a
    base58-encoded string. It can easily be output as UUID or bytes using ULID methods.

    NOTE: IDs that are generated in Python code do not guarantee monotonicity, because
    the underlying python-ulid library does not. To test this try the following:

    ```python
    # generate a list of indexed IDs, sort that list by ID, and compare the two
    >>> import json; from blackledger.domain.types import ID
    >>> l0 = [(i, ID()) for i in range(int(10))]
    >>> l1 = list(sorted(l0, key=lambda i: i[1]))
    >>> l1 == l0
    False
    >>> l1
    [(0, ID('C7garHDQEHXkq2oxPzwC6')),
     (3, ID('C7garHDTLFc4UMZnWESBG')),
     (5, ID('C7garHDTgra76mCT13fp7')),
     (4, ID('C7garHDVTxWrHkBsLFN6X')),
     (6, ID('C7garHDWNLBuHJEB6HwU3')),
     (8, ID('C7garHDWvtVoX6pAHYYAF')),
     (2, ID('C7garHDXL75t88gZuaTHZ')),
     (9, ID('C7garHDYkU1tWdvKPNi7s')),
     (1, ID('C7garHDZqcfLXp4PZoku5')),
     (7, ID('C7garHDajsKSogFn1S3DH'))]
    ```

    To guarantee monotonicity, you need a global sequence that is guaranteed to increase
    for each generated instance. The blackledger database provides a `gen_id(SEQUENCE)`
    function that does guarantee monotonicity for all ids generated with that sequence
    (caveats below). Example usage (in psql, the PostgreSQL shell):

    ```sql
        blackledger=# CREATE SEQUENCE my_id_seq AS smallint CYCLE;
        CREATE SEQUENCE
        blackledger=# select gen_id('my_id_seq') from generate_series(1, 5);
                        gen_id
        --------------------------------------
         018d4cb4-6c37-0001-6998-f9d601f92b06
         018d4cb4-6c37-0002-4a7c-4e22522a95b0
         018d4cb4-6c37-0003-6178-a7c3333e6798
         018d4cb4-6c37-0004-5a67-8ae96a76bd2c
         018d4cb4-6c37-0005-4915-d3703bbf7a0f
    ```
    The IDs are stored in the database as UUIDs but are presented as encoded IDs in the
    blackledger app. Most tables in blackledger use the gen_id(SEQUENCE_NAME) function
    to generate ids for all records.

    The IDs generated with gen_id(SEQUENCE_NAME) use a global sequence after the
    timestamp to guarantee that the IDs are always generated sequentially
    (monotonically). To do the same in Python would similarly require a global sequence.
    But doing so is at odds with having a stateless multi-instance application. So any
    use that requires monotonic IDs should generate them from the database:

    ```python
    >>>  import psycopg; from blackledger.settings import DatabaseSettings
    >>> db = DatabaseSettings(); sql = SQL(dialect=db.dialect)
    >>> conn = psycopg.connect(conninfo=db.url.get_secret_value())
    >>> records = sql.select_all(conn,
    ...     "select i, gen_id('my_id_seq') id from generate_series(1,5) i")
    >>> [(r['i'], r['id'].to_uuid()) for r in records]
    [(1, UUID('018d4ced-3a4e-0001-3a42-804c43e425cd')),
     (2, UUID('018d4ced-3a4e-0002-0402-e203134921f0')),
     (3, UUID('018d4ced-3a4e-0003-4709-6d982d900a58')),
     (4, UUID('018d4ced-3a4e-0004-5bde-461b49abf7d6')),
     (5, UUID('018d4ced-3a4e-0005-7710-ef801d337205'))]
    ```

    CAVEAT: Generating IDs in the database is slower than generating IDs in Python.

    CAVEAT: To guarantee monotonicity, the `gen_id(...)` sequence must not wrap around
    during the same millisecond. (For example: `generate_series(...)` operates during a
    single timestamp.) To ensure that the ids generated using `gen_id(...)` increase
    monotonically, set the start value of the sequence and generate fewer ids during a
    query (or within the same millisecond) than can be accommodated within sequence max.

    ```sql
    blackledger=# select setval('my_id_seq', 32760);  /* simulate many past ids */
    blackledger=# select i, gen_id('my_id_seq') id from generate_series(1, 10) i;
     i  |                  id
    ----+--------------------------------------
      1 | 018d4cf3-9d3a-7ff9-01a4-52290f4a438e
      2 | 018d4cf3-9d3a-7ffa-35cb-d78362e5ebb9
      3 | 018d4cf3-9d3a-7ffb-5ab7-3aeb5f0bfc35
      4 | 018d4cf3-9d3a-7ffc-3907-d07905097bf6
      5 | 018d4cf3-9d3a-7ffd-537e-75c33926c271
      6 | 018d4cf3-9d3a-7ffe-1c74-70ac3fe5f11c
      7 | 018d4cf3-9d3a-7fff-15a7-cb6e3c3487cf
      8 | 018d4cf3-9d3a-0001-6ee2-1bfc09d541eb  /* <-- WHOOPS! wrap-around! */
      9 | 018d4cf3-9d3a-0002-5dcb-aff21f5ef447
     10 | 018d4cf3-9d3a-0003-5b84-bc7f5d71d66e

    blackledger=# select setval('my_id_seq', 32767);  /* set it to the max seq value */
    blackledger=# select i, gen_id('my_id_seq') id from generate_series(1, 10) i;
     i  |                  id
    ----+--------------------------------------
      1 | 018d4cf6-13fc-0001-273d-d2242c7aebef
      2 | 018d4cf6-13fc-0002-02f0-c00a4b1c358a
      3 | 018d4cf6-13fc-0003-34e7-91fc6a6c1729
      4 | 018d4cf6-13fc-0004-1ace-21220bdebcbf
      5 | 018d4cf6-13fc-0005-5f60-adb554e39c23
      6 | 018d4cf6-13fc-0006-6036-074077956f4b
      7 | 018d4cf6-13fc-0007-38de-d88d6f9021bf
      8 | 018d4cf6-13fc-0008-2125-79392069ea72
      9 | 018d4cf6-13fc-0009-5309-67156dcdb0ee
     10 | 018d4cf6-13fc-000a-30a6-bbd016e8dbb1  /* no wrap-around, hooray */
    ```
    """

    def __init__(self, value: Optional[str] = None):
        """
        If a value is given, it is a base58 encoded string to initialize ID. Otherwise,
        a new ID is generated via ULID.
        """
        if value:
            super().__init__(ULID(base58.b58decode(value)).hex)
        else:
            super().__init__(ULID().hex)

    def __str__(self):
        """
        ID is represented as a base58-encoded string
        """
        return base58.b58encode(self.bytes).decode()

    @classmethod
    def from_uuid(cls, value: str | UUID):
        if isinstance(value, str):
            value = UUID(value)
        return cls(base58.b58encode(value.bytes))

    @classmethod
    def field_converter(cls, value):
        if isinstance(value, str):
            value = cls(value)
        return value

    def to_uuid(self):
        return UUID(self.hex)
