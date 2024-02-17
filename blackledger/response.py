import json
from typing import Any

import fastapi.responses

from blackledger import types


class JsonEncoder(json.JSONEncoder):
    def default(self, obj: Any):
        if isinstance(obj, (types.ID,)):
            return str(obj)
        else:
            return super().default(obj)


class JSONResponse(fastapi.responses.JSONResponse):
    def render(self, content: Any) -> bytes:
        return json.dumps(
            content,
            ensure_ascii=False,
            allow_nan=False,
            indent=None,
            separators=(",", ":"),
            cls=JsonEncoder,
        ).encode("utf-8")
