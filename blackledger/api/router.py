from fastapi import APIRouter, Depends

from blackledger.auth import JWTAuthorization
from blackledger.meta import __name__, __version__
from blackledger.settings import AuthSettings

from . import accounts, currencies, ledgers, transactions

jwt_authorization_dependency = JWTAuthorization(AuthSettings(), name="Authorization")
router = APIRouter(dependencies=[Depends(jwt_authorization_dependency)])

router.include_router(ledgers.router)
router.include_router(currencies.router)
router.include_router(accounts.router)
router.include_router(transactions.router)


@router.get("", tags=["home"])
async def home():
    return {
        "name": __name__,
        "version": __version__,
    }
