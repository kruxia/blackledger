from fastapi import APIRouter

from blackledger.api import accounts, currencies, transactions
from blackledger.meta import __name__, __version__

api_router = APIRouter()


@api_router.get("")
async def home():
    return {
        "name": __name__,
        "version": __version__,
    }


api_router.include_router(accounts.router)
api_router.include_router(currencies.router)
api_router.include_router(transactions.router)
