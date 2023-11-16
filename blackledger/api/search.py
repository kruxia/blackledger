from typing import Optional

from pydantic import BaseModel, constr
from sqly import Q


class SearchFilters(BaseModel):
    @classmethod
    def from_query(cls, qargs):
        return cls(**{k: v for k, v in qargs.items() if not k.startswith("_")})

    def select_filters(self):
        data = self.dict(exclude_none=True)
        return [
            Q.filter(key, op="~*" if isinstance(val, str) else "=")
            for key, val in data.items()
        ]

    def query_data(self):
        return self.dict(exclude_none=True)


class SearchParams(BaseModel):
    orderby: Optional[constr(pattern=r"^-?\w+(,\-?\w+)*$")] = None
    limit: Optional[int] = None
    offset: Optional[int] = None

    @classmethod
    def from_query(cls, qargs):
        """
        Convert query args to (SQL) search parameters
        """
        return cls(**{k.lstrip("_"): v for k, v in qargs.items() if k.startswith("_")})

    def select_params(self):
        data = self.dict(exclude_none=True, exclude=["orderby"])
        if self.orderby:
            data["orderby"] = ",".join(
                [
                    f"{field.strip('-')} desc" if field.startswith("-") else field
                    for field in [field.strip() for field in self.orderby.split(",")]
                ]
            )
        return data
