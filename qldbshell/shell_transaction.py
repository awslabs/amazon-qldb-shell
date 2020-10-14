from botocore.exceptions import ClientError

from .outcome import Outcome
from qldbshell.shell_utils import print_result
import logging


class ShellTransaction:
    """
    Responsible for storing and executing the queries and
    outcome of a transaction. Additionally, tracks if a
    start transaction is needed.
    """

    def __init__(self, outcome):
        self._queries = []
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
                logging.info("Query: {}".format(query))
                print_result(driver_transaction.execute_statement(query))
            except ClientError as ce:
                driver_transaction.abort()
                raise ce

    def execute_outcome(self, driver_transaction):
        transaction_id = driver_transaction.transaction_id
        if self._outcome == Outcome.ABORT:
            driver_transaction.abort()
            logging.info("Transaction with transaction id {} aborted".format(transaction_id))
        elif self._outcome == Outcome.COMMIT:
            driver_transaction.commit()
            logging.info("Transaction with transaction id {} committed".format(transaction_id))
        else:
            return
        return

    def set_outcome(self, outcome):
        self._outcome = outcome
        self._start = False

    def add_query(self, query):
        self._queries.append(query)
        self._start = True


