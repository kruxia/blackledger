from fastapi import APIRouter

router = APIRouter(prefix="/accounts")


@router.get("")
async def search_accounts():
    """
    Search for and list accounts.
    """
    raise NotImplementedError("Account search")


@router.post("")
async def edit_accounts():
    """
    Insert/update accounts.
    """
    raise NotImplementedError("Account editing")
