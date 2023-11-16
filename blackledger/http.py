import psycopg_pool
from fastapi import FastAPI
from fastapi.responses import JSONResponse
from pydantic import ValidationError
from sqly import ASQL

from blackledger.api.__router__ import api_router
from blackledger.settings import DatabaseSettings

app = FastAPI()
app.include_router(api_router, prefix="/api")


@app.exception_handler(NotImplementedError)
async def not_implemented_error_handler(_, exc: NotImplementedError):
    return JSONResponse(
        status_code=404,
        content={"message": f"{str(exc)}: Not implemented"},
    )


@app.exception_handler(ValidationError)
async def validation_error_handler(_, exc: ValidationError):
    return JSONResponse(
        status_code=412,
        content=exc.errors(),
    )


@app.on_event("startup")
async def app_on_startup():
    # create the database pool and SQL query engine
    db = DatabaseSettings()
    app.sql = ASQL(dialect=db.dialect)
    app.pool = psycopg_pool.AsyncConnectionPool(
        conninfo=db.url.get_secret_value(), open=False
    )
    await app.pool.open()


@app.on_event("shutdown")
async def app_on_shutdown():
    # close the database pool
    await app.pool.close()
