from typing import Optional

from pydantic import Field
from sqly import Q

from blackledger.model import Model


class SearchParams(Model):
    orderby: Optional[str] = Field(
        default=None,
        pattern=r"^-?\w+(,\-?\w+)*$",
        alias="_orderby",
        examples=["FIELD_NAME"],
    )
    limit: Optional[int] = Field(default=100, le=100, alias="_limit")
    offset: Optional[int] = Field(default=None, alias="_offset")

    def select_params(self):
        params = {
            k.lstrip("_"): v
            for k, v in self.model_dump(exclude_none=True, by_alias=True).items()
            if k.startswith("_")
        }
        if self.orderby:
            params["orderby"] = ",".join(
                [
                    f"{field.lstrip('-')} desc" if field.startswith("-") else field
                    for field in [field.strip() for field in self.orderby.split(",")]
                ]
            )
        return params

    def select_filters(self):
        return [
            (
                Q.ANY(key)
                if isinstance(val, list)
                else Q.filter(
                    key,
                    op="~*" if isinstance(val, str) else "=",
                )
            )
            for key, val in self.query_data().items()
        ]

    def query_data(self):
        return {
            k: v
            for k, v in self.model_dump(exclude_none=True, by_alias=True).items()
            if not k.startswith("_")
        }
