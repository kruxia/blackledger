from fastapi import APIRouter

from blackledger.api import accounts, currencies, tenants, transactions
from blackledger.meta import __name__, __version__

router = APIRouter()


@router.get("", tags=["home"])
async def home():
    return {
        "name": __name__,
        "version": __version__,
    }


router.include_router(accounts.router)
router.include_router(currencies.router)
router.include_router(tenants.router)
router.include_router(transactions.router)
