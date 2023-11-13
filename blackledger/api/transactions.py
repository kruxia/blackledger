from fastapi import APIRouter

router = APIRouter(prefix="/transactions")


@router.get("")
async def search_transactions():
    """
    Search for and list transactions.
    """
    raise NotImplementedError("Transaction search")


@router.post("")
async def post_transactions():
    """
    Post transactions.
    """
    raise NotImplementedError("Transaction posting")
