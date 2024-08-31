import logging
import re
import traceback
from http import HTTPStatus

import jwt
from fastapi import HTTPException
from fastapi.security import APIKeyHeader
from starlette.requests import Request

LOG = logging.getLogger(__name__)


class JWTAuthorization(APIKeyHeader):
    """
    Middleware that protects every request in the router by ensuring that the request
    client has a valid (signed) token (JWT) in the Authorization header.
    """

    TOKEN_PATTERN = re.compile(r"^.*Bearer\s*(.*)$", flags=re.I)

    def __init__(self, auth_settings, **kwargs):
        super().__init__(**kwargs)
        self.settings = auth_settings
        self.client = jwt.PyJWKClient(self.settings.jwks_url)

    async def __call__(self, request: Request):
        if request.app.settings.auth.disabled is True:
            return
        try:
            bearer = request.headers.get("Authorization")
            assert bearer is not None, "Authorization header not found"
            request.state.auth_claims = self.decode(bearer)
        except Exception as exc:
            LOG.warning(traceback.format_exc())
            raise HTTPException(
                status_code=HTTPStatus.UNAUTHORIZED,
                detail=f"Unauthorized: {exc.__class__.__name__}: {str(exc)}",
            )

    def decode(self, bearer) -> dict:
        """
        If the signature and required claims of the (identity) token are valid, decode
        the token and return a dict of the claims it contains. Raise an Exception if the
        signature is invalid, the required claims do not match expectations, the token
        is not yet valid, or it has expired.
        """
        token = re.sub(self.TOKEN_PATTERN, r"\1", bearer)
        key = self.client.get_signing_key_from_jwt(token).key
        claims = jwt.decode(
            token,
            key,
            algorithms=[self.settings.algorithm],
            issuer=self.settings.issuer,
            options=dict(
                require=["iss", "iat", "exp", "sub"],
                verify_signature=True,
                verify_aud=bool(self.settings.audience),  # only verify if configured
                verify_exp=True,
                verify_iss=True,
            ),
        )
        return claims
