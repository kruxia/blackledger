from pathlib import Path
from fastapi import APIRouter, Request
from fastapi.responses import HTMLResponse
from fastapi.templating import Jinja2Templates

from blackledger.meta import __name__, __version__
from blackledger.domain import model, types

PATH = Path(__file__).absolute().parent

app = APIRouter()
templates = Jinja2Templates(
    directory=PATH / "templates",
    # adjusting the default template variable syntax `{{ ... }}` in order to avoid
    # conflicts with VueJS
    variable_start_string="{$",
    variable_end_string="}",
)


@app.get("/", response_class=HTMLResponse)
@app.get("/{tenant_id}/{template_path:path}", response_class=HTMLResponse)
async def home(req: Request, tenant_id: str = None, template_path: str = "home"):
    print(f"{template_path=}")
    return templates.TemplateResponse(f"{template_path}.html", {"request": req, "tenant_id": tenant_id})
