import os

import psycopg
from sqly import SQL, queries

from blackledger.database import type_adapters  # noqa
from blackledger.domain import model

sql = SQL(dialect="psycopg")

conn = psycopg.connect(os.environ["DATABASE_URL"])
curs = conn.execute("select * from accounts where name='Asset'")
record = dict(zip((d.name for d in curs.description), curs.fetchone()))
parent = model.Account(**record)
account = model.Account(name="bank", currency="USD", parent=parent, normal="CR")
qd = dict(account)
qd["parent"] = account.parent.id
query, params = sql.render(queries.INSERT("accounts", qd) + "RETURNING *", qd)
print(f"{query=}")
print(f"{params=}")
curs = conn.execute(query, params)
record = dict(zip((d.name for d in curs.description), curs.fetchone()))
print(f"{record=}")
