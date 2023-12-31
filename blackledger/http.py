from contextlib import asynccontextmanager
from http import HTTPStatus
from pathlib import Path

import psycopg_pool
from fastapi import FastAPI
from fastapi.responses import JSONResponse
from fastapi.staticfiles import StaticFiles
from psycopg.errors import ForeignKeyViolation, RaiseException, UniqueViolation
from pydantic import ValidationError
from sqly import ASQL

from blackledger import api, ui
from blackledger.db import type_adapters  # noqa - provides psycopg registrations.
from blackledger.settings import DatabaseSettings


@asynccontextmanager
async def lifespan(app: FastAPI):
    # create the database pool and SQL query engine
    db = DatabaseSettings()
    app.sql = ASQL(dialect=db.dialect)
    app.pool = psycopg_pool.AsyncConnectionPool(
        conninfo=db.url.get_secret_value(), open=False
    )
    await app.pool.open()

    yield

    # close the database pool
    await app.pool.close()


PATH = Path(__file__).absolute().parent

app = FastAPI(lifespan=lifespan)
app.mount("/static", StaticFiles(directory=PATH / "ui" / "static"), name="static")
app.include_router(api.router, prefix="/api")
app.include_router(ui.app)


@app.exception_handler(NotImplementedError)
async def not_implemented_error_handler(_, exc: NotImplementedError):
    return JSONResponse(
        status_code=HTTPStatus.NOT_FOUND,
        content={"message": f"{str(exc)}: Not implemented"},
    )


@app.exception_handler(ValidationError)
async def validation_error_handler(_, exc: ValidationError):
    errors = exc.errors()
    for error in errors:
        for key in error.get("ctx", {}):
            error["ctx"][key] = str(error["ctx"][key])
    return JSONResponse(
        status_code=HTTPStatus.PRECONDITION_FAILED,
        content=errors,
    )


@app.exception_handler(UniqueViolation)
@app.exception_handler(ForeignKeyViolation)
@app.exception_handler(RaiseException)  # immutable fields raise this
async def unique_violation_error_handler(_, exc: Exception):
    return JSONResponse(
        status_code=HTTPStatus.CONFLICT,
        content={"message": str(exc)},
    )
