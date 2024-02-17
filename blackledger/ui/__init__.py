from pathlib import Path

import jinja2.exceptions
from fastapi import APIRouter, Request
from fastapi.exceptions import HTTPException
from fastapi.responses import HTMLResponse
from fastapi.templating import Jinja2Templates

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
    try:
        response = templates.TemplateResponse(
            f"{template_path}.html", {"request": req, "tenant_id": tenant_id}
        )
    except jinja2.exceptions.TemplateNotFound:
        raise HTTPException(
            status_code=404, detail=f"Template not found: {template_path}"
        )

    return response
