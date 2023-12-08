from pathlib import Path
from fastapi import APIRouter, Request
from fastapi.responses import HTMLResponse
from fastapi.templating import Jinja2Templates

from blackledger.meta import __name__, __version__
PATH = Path(__file__).absolute().parent

app = APIRouter()
templates = Jinja2Templates(directory=PATH / "templates")

@app.get("/", response_class=HTMLResponse)
@app.get("/{path}", response_class=HTMLResponse)
async def home(req: Request, path: Path = "home"):
    return templates.TemplateResponse(f"{path}.html", {"request": req})
