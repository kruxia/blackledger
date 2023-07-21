from dataclasses import InitVar, dataclass

from psycopg import Connection
from sqly import SQL, Dialect, queries

from blackledger.domain import model, types


@dataclass
class Accounts:
    dialect: InitVar[Dialect]
    sql: SQL = None
    table: str = "accounts"

    def __post_init__(self, dialect):
        self.sql = SQL(dialect=dialect)

    def insert(self, conn: Connection, item: model.Account):
        data = dict(item)
        data["parent"] = item.parent.id if item.parent else None
        query, params = self.sql.render(queries.INSERT(self.table, data), data)
        with conn.cursor() as cursor:
            cursor.execute(query, params)

    def search(
        self, conn: Connection, filters=None, limit=None, orderby=None, offset=None
    ):
        query, _ = self.sql.render(
            queries.SELECT(
                self.table, filters=filters, limit=limit, orderby=orderby, offset=offset
            )
        )
        with conn.cursor() as cursor:
            cursor.execute(query)
            fields = [d.name for d in cursor.description]
            for result in cursor:
                record = {k: v for k, v in dict(zip(fields, result)).items()}
                # ignore parent for now
                record.pop("parent")
                # cast UUID to ID
                record["id"] = types.ID.from_uuid(record["id"])

                yield model.Account(**record)


@dataclass
class Transactions:
    dialect: InitVar[Dialect]
    sql: SQL = None
    table: str = "transactions"

    def __post_init__(self, dialect):
        self.sql = SQL(dialect=dialect)

    def insert(self, conn: Connection, item: model.Transaction):
        data = dict(item)
        data["parent"] = item.parent.id if item.parent else None
        query, params = self.sql.render(queries.INSERT(self.table, data), data)
        with conn.cursor() as cursor:
            cursor.execute(query, params)

    def search(
        self, conn: Connection, filters=None, limit=None, orderby=None, offset=None
    ):
        query, _ = self.sql.render(
            queries.SELECT(
                self.table, filters=filters, limit=limit, orderby=orderby, offset=offset
            )
        )
        with conn.cursor() as cursor:
            cursor.execute(query)
            fields = [d.name for d in cursor.description]
            for result in cursor:
                record = {k: v for k, v in dict(zip(fields, result)).items()}
                # ignore parent for now
                record.pop("parent")
                # cast UUID to ID
                record["id"] = types.ID.from_uuid(record["id"])

                yield model.Transaction(**record)
