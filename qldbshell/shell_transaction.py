from botocore.exceptions import ClientError

from .outcome import Outcome
from qldbshell.shell_utils import print_result


class ShellTransaction:

    def __init__(self, queries, outcome):
        self._queries = queries
        self._outcome = outcome
        self._start = True

    def get_outcome(self):
        return self._outcome

    def is_start(self):
        return self._start and (len(self._queries) == 0)

    def is_open(self):
        return self._start and (len(self._queries) > 0)

    def run_transaction(self, driver_transaction):
        for query in self._queries:
            try:
                print_result(driver_transaction.execute_statement(query))
            except ClientError as ce:
                driver_transaction.abort()
                raise ce

    def execute_outcome(self, driver_transaction):
        if self._outcome == Outcome.ABORT:
            driver_transaction.abort()
        elif self._outcome == Outcome.COMMIT:
            driver_transaction.commit()
        else:
            return
        return

    def set_outcome(self, outcome):
        self._outcome = outcome
        self._start = False

    def add_query(self, query):
        self._queries.append(query)
        self._start = True

