from http import HTTPStatus


def test_home(client):
    response = client.get("/api")
    assert response.status_code == HTTPStatus.OK
