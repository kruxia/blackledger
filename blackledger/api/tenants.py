from typing import Annotated, Optional

from fastapi import APIRouter, Depends, Request

from blackledger.db import queries
from blackledger.domain import model, types

from ._search import SearchFilters, SearchParams

router = APIRouter(prefix="/tenants", tags=["tenants"])


class TenantFilters(SearchFilters):
    # allow partial match where applicable
    id: Optional[model.IDField] = None
    name: Optional[types.NameFilter] = None


@router.get("", response_model=list[model.Tenant])
async def search_tenants(
    req: Request, filters: Annotated[TenantFilters, Depends(TenantFilters)]
):
    """
    Search for and list tenants.
    """
    params = SearchParams.from_query(req.query_params)
    async with req.app.pool.connection() as conn:
        results = await queries.select_tenants(conn, req.app.sql, filters, params)

    return results


@router.post("", response_model=model.Tenant)
async def edit_tenants(req: Request, item: model.Tenant):
    """
    Insert/update tenant.
    """
    sql = req.app.sql
    data = item.model_dump(exclude_unset=True)
    query = sql.queries.UPSERT("tenant", fields=data, key=["id"], returning=True)
    async with req.app.pool.connection() as conn:
        result = await sql.select_one(conn, query, data, Constructor=model.Tenant)
    return result
