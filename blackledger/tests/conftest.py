import pytest

from blackledger.tests import fixtures

dbpool = pytest.fixture(scope="session")(fixtures.dbpool)
sql = pytest.fixture(scope="session")(fixtures.sql)
json_dumps = pytest.fixture(scope="session")(fixtures.json_dumps)

# == DATA FIXTURES ==
base_currencies = pytest.fixture(scope="session", autouse=True)(
    fixtures.base_currencies
)
base_tenant_name = pytest.fixture(scope="session", autouse=True)(
    fixtures.base_tenant_name
)
base_tenant = pytest.fixture(scope="session", autouse=True)(fixtures.base_tenant)

# -- Test-scoped fixtures --
client = pytest.fixture(scope="function")(fixtures.client)
auth_client = pytest.fixture(scope="function")(fixtures.auth_client)
run_id = pytest.fixture(scope="function")(fixtures.run_id)
test_accounts = pytest.fixture(scope="function")(fixtures.test_accounts)
test_transactions = pytest.fixture(scope="function")(fixtures.test_transactions)
