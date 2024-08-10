import pytest

from blackledger.tests import fixtures

dbpool = pytest.fixture(scope="session")(fixtures.dbpool)
sql = pytest.fixture(scope="session")(fixtures.sql)
json_dumps = pytest.fixture(scope="session")(fixtures.json_dumps)

# == DATA FIXTURES ==
base_currencies = pytest.fixture(scope="session", autouse=True)(
    fixtures.base_currencies
)
base_ledger_name = pytest.fixture(scope="session", autouse=True)(
    fixtures.base_ledger_name
)
base_ledger = pytest.fixture(scope="session", autouse=True)(fixtures.base_ledger)

# -- Test-scoped fixtures --
client = pytest.fixture(scope="function")(fixtures.client)
auth_client = pytest.fixture(scope="function")(fixtures.auth_client)
run_id = pytest.fixture(scope="function")(fixtures.run_id)
test_accounts = pytest.fixture(scope="function")(fixtures.test_accounts)
test_transactions = pytest.fixture(scope="function")(fixtures.test_transactions)
