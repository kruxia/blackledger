def select_account_w_path(
    account_fields=["*"], filters=None, orderby=None, limit=None, offset=None
):
    query = f"""
        with recursive nodes(id, path) as (
            select id, name
            from account
            where parent_id is null
        union all
            select a.id, a.concat(path, ' : ', name)
            from account a
            join nodes n on n.id = a.parent_id
        )
        select n.path, {','.join(f'a.{field}' for field in account_fields)}
        from nodes n
        join account a on n.id = a.id
    """
    if filters:
        query += f"WHERE {' AND '.join(filters)}"
    if orderby:
        query += f"ORDER BY {orderby}"
    if limit:
        query += f"LIMIT {limit}"
    if offset:
        query += f"OFFSET {offset}"

    return query


def select_account_w_descendants(fields=["*"]):
    query = f"""
        with recursive nodes(id) as (
            select id
            from account
            where id = :id
        union all
            select a.id
            from account a
            join nodes n on n.id = a.parent_id
        )
        select {",".join(f"a.{field}" for field in fields)}
        from account a
        join nodes n on a.id = n.id
    """

    return query
