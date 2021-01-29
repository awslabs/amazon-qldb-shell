# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
#
# Licensed under the Apache License, Version 2.0 (the "License").
# You may not use this file except in compliance with the License.
# A copy of the License is located at
#
# http://www.apache.org/licenses/LICENSE-2.0
#
# or in the "license" file accompanying this file. This file is distributed
# on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either
# express or implied. See the License for the specific language governing
# permissions and limitations under the License.

import logging

from .command_container import Command, CommandContainer
from .errors import IllegalStateError
from .shell_utils import print_result


class ShellTransaction:
    """
    Responsible for storing and executing the queries and
    outcome of a transaction.
    """

    def __init__(self, outcome):
        self._queries = []
        self.outcome = outcome

    def run_transaction(self, statement_queue, result_queue):
        for query in self._queries:
            logging.info("Query: {}".format(query))
            statement_queue.put(CommandContainer(Command.EXECUTE, statement=query))
            container = result_queue.get()
            result = container.output
            if container.command != Command.EXECUTE:
                raise IllegalStateError("Invalid state due to an unexpected command result")
            print_result(result)

    def execute_outcome(self, transaction_id, statement_queue, result_queue):
        if self.outcome == Command.ABORT:
            statement_queue.put(CommandContainer(Command.ABORT))
            container = result_queue.get()
            if container.command != Command.ABORT:
                raise IllegalStateError("Invalid state due to an unexpected command result")
            logging.info("Transaction with transaction id {} aborted".format(transaction_id))
        elif self.outcome == Command.COMMIT:
            statement_queue.put(CommandContainer(Command.COMMIT))
            container = result_queue.get()
            try:
                container.output
            except Exception as e:
                logging.info("Transaction with transaction id {} could not be committed".format(transaction_id))
                raise e
            if container.command != Command.COMMIT:
                raise IllegalStateError("Invalid state due to an unexpected command result")
            logging.info("Transaction with transaction id {} committed".format(transaction_id))

    def add_query(self, query):
        self._queries.append(query)
