from http import HTTPStatus
from unittest.mock import MagicMock, patch


class MockPyJWKClient(MagicMock):
    get_signing_key_from_jwt = MagicMock()


def test_home_auth_unauthorized(auth_client):
    response = auth_client.get("/api")
    assert response.status_code == HTTPStatus.UNAUTHORIZED


@patch("blackledger.api.router.jwt_authorization_dependency.client", MockPyJWKClient())
@patch("jwt.decode", MagicMock(return_value={}))
def test_home_auth_authorized(auth_client):
    response = auth_client.get("/api", headers={"Authorization": "CAN_HAZ"})
    assert response.status_code == HTTPStatus.OK
