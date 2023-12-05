from decimal import Decimal
from typing import Optional

from fastapi import APIRouter, Request
from pydantic import ConfigDict, constr, field_serializer, field_validator

from blackledger.domain import model, types

from ._search import SearchFilters, SearchParams

router = APIRouter(prefix="/tenants")


class TenantFilters(SearchFilters):
    # allow partial match where applicable
    id: Optional[model.ID] = None
    name: Optional[constr(pattern=r"^\S+$")] = None

    model_config = ConfigDict(arbitrary_types_allowed=True)


@router.get("", response_model=list[model.Tenant])
async def search_tenants(req: Request):
    """
    Search for and list tenants.
    """
    filters = TenantFilters.from_query(req.query_params)
    params = SearchParams.from_query(req.query_params).select_params()
    query = req.app.sql.queries.SELECT(
        "tenant", filters=filters.select_filters(), **params
    )
    async with req.app.pool.connection() as conn:
        results = await req.app.sql.select_all(
            conn, query, filters.query_data(), Constructor=model.Tenant
        )

    return results


@router.post("", response_model=model.Tenant)
async def edit_tenants(req: Request, item: model.Tenant):
    """
    Insert/update tenant.
    """
    sql = req.app.sql
    data = item.model_dump(exclude_none=True)
    query = sql.queries.UPSERT("tenant", fields=data, key=["id"], returning=True)
    async with req.app.pool.connection() as conn:
        result = await sql.select_one(conn, query, data, Constructor=model.Tenant)
    return result
