import logging

from sqly import SQL

from blackledger import model, search

LOG = logging.getLogger(__name__)


def _get_where_clause(params: search.SearchParams) -> list[str]:
    select_filters = params.select_filters()
    query_filters = []
    if select_filters:
        query_filters += [
            "WHERE",
            " AND ".join(select_filters),
        ]
    return query_filters


def _get_query_params(params: search.SearchParams) -> list[str]:
    select_params = params.select_params()
    select_args = []
    if select_params.get("orderby"):
        select_args.append(f"ORDER BY {select_params['orderby']}")
    if select_params.get("limit"):
        select_args.append(f"LIMIT {select_params['limit']}")
    if select_params.get("offset"):
        select_args.append(f"OFFSET {select_params['offset']}")
    return select_args


async def select_balances(conn, sql: SQL, params: search.SearchParams):
    query = [
        """
        WITH balances AS (
            SELECT a.id account_id,
                e.curr, sum(e.dr) dr, sum(e.cr) cr
            FROM account a
            JOIN entry e
                ON a.id = e.acct
            GROUP BY (a.id, e.curr)
        )
        SELECT account.*,
            balances.curr, balances.dr, balances.cr
        FROM account
        JOIN balances
            ON account.id = balances.account_id
        """
    ]
    query.extend(_get_where_clause(params))
    query.extend(_get_query_params(params))
    LOG.debug(sql.render(query, params.query_data()))

    results = await sql.select_all(conn, query, params.query_data())

    return results


async def select_currencies(conn, sql: SQL, params: search.SearchParams):
    query = sql.queries.SELECT(
        "currency", filters=params.select_filters(), **params.select_params()
    )
    LOG.debug(sql.render(query, params.query_data()))
    results = await sql.select_all(
        conn,
        query,
        params.query_data(),
        Constructor=model.Currency,
    )
    return results


async def select_ledgers(conn, sql: SQL, params: search.SearchParams):
    query = sql.queries.SELECT(
        "ledger", filters=params.select_filters(), **params.select_params()
    )
    LOG.debug(sql.render(query, params.query_data()))
    results = await sql.select_all(
        conn, query, params.query_data(), Constructor=model.Ledger
    )
    return results


async def select_transactions(conn, sql: SQL, params: search.SearchParams):
    # select the transactions
    tx_query = [
        # filter transaction ids based on transaction and entry fields
        "WITH filtered_tr AS (",
        "  SELECT distinct(transaction.id)",
        "  FROM transaction",
        "  JOIN entry",
        "    ON transaction.id = entry.tx",
    ]
    tx_query.extend(_get_where_clause(params))
    tx_query += [
        ")",
        # select matching transactions (only -- entries are separate)
        "SELECT",
        "  DISTINCT tx.*",
        "FROM transaction tx",
        "JOIN filtered_tr",
        "  ON filtered_tr.id = tx.id",
    ]
    tx_query.extend(_get_query_params(params))
    LOG.debug("\n%s\n\t%r" % sql.render(tx_query, params.query_data()))

    tx_results = await sql.select_all(
        conn, tx_query, params.query_data(), Constructor=model.Transaction
    )

    # construct a dictionary of transactions by id
    transactions = {tx.id: tx for tx in tx_results}

    # select corresponding entries
    entries_query = """
        SELECT e.*, a.name acct_name FROM entry e
        JOIN transaction t ON e.tx = t.id
        JOIN account a ON e.acct = a.id
        WHERE t.id = ANY(:tx)
    """
    entries_params = {"tx": [str(tx_id.to_uuid()) for tx_id in transactions.keys()]}
    LOG.debug("\n%s\n\t%r" % sql.render(entries_query, entries_params))

    entries_results = await sql.select_all(
        conn, entries_query, entries_params, Constructor=model.Entry
    )

    # collate entries under their transactions
    for entry in entries_results:
        transactions[entry.tx].entries.append(entry)

    # select queries return lists
    return list(transactions.values())
