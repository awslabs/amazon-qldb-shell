# Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
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
import cmd
import logging
from textwrap import dedent


from botocore.exceptions import ClientError, EndpointConnectionError, NoCredentialsError
from pyqldb.errors import SessionPoolEmptyError

from qldbshell.decorators import (time_this, zero_noun_command)

from . import version
from .outcome import Outcome
from .errors import NoCredentialError, QuerySyntaxError
from .errors import is_transaction_expired_exception
from .shell_transaction import ShellTransaction

from qldbshell.shell_utils import print_result


class QldbShell(cmd.Cmd):

    def __init__(self, profile="default", pooled_driver=None):
        super(QldbShell, self).__init__()
        if profile:
            print(profile)
        print()

        self._driver = pooled_driver
        try:
            session = self._driver.get_session()
            session.close()
        except NoCredentialsError:
            raise NoCredentialError("No credentials present") from None
        self._is_interactive_transaction = False
        self._driver_transaction = None
        self._transaction_session = None


    prompt = 'qldbshell > '

    intro = dedent(f"""\
        Welcome to the Amazon QLDB Shell version {version}
        Use 'start' to initiate and interact with a transaction. 'commit' and 'abort' to commit or abort a transaction.
        Use 'start; statement 1; statement 2; commit; start; statement 3; commit' to create transactions non-interactively.
        All other commands will be interpreted as PartiQL statements until the 'exit' command is issued.
        
    """)

    def onecmd(self, line):
        try:
            return super().onecmd(line)
        except EndpointConnectionError as e:
            logging.fatal(f'Unable to connect to an endpoint. Please check Shell configuration. {e}')
            self.quit_shell()
        except SessionPoolEmptyError as e:
            logging.info(f'Query failed, please try again')
        except ClientError as e:
            logging.error(f'Error encountered: {e}')
        return False # don't stop

    def do_EOF(self, line):
        'Exits the Shell; equivalent to calling quit: EOF'
        self.quit_shell()

    def quit_shell(self):
        logging.info("Exiting qldb shell.")
        exit(0)

    @zero_noun_command
    def do_exit(self, line):
        'Exit the qldb shell: quit'
        self.quit_shell()

    @time_this
    def default(self, line):
        if line.strip().lower().startswith("start") or self._is_interactive_transaction:
            self.handle_transaction_flow(line)
        elif (self._is_interactive_transaction is False) and (line.lower().strip().strip(";") == "abort"):
            logging.info("'abort' can only be used on an active transaction")
        elif (self._is_interactive_transaction is False) and (line.lower().strip().strip(";") == "commit"):
            logging.info("'commit' can only be used on an active transaction")
        else:
            session = self._driver.get_session()
            try:
                print_result(session.execute_lambda(
                    lambda x: x.execute_statement(line)))
            except ClientError as e:
                logging.warning(f'Error while executing query: {e}')
            finally:
                session.close()

    def handle_transaction_flow(self, line):
        try:
            shell_transactions = self.process_input(line)
            self.run_transactions(shell_transactions)
        except QuerySyntaxError as qse:
            logging.warning(f'Error in query: {qse}')

        except ClientError as ce:
            if is_transaction_expired_exception(ce):
                logging.info("Transaction expired.")
            else:
                logging.warning(f'Error in query: {ce}')
            self.close_interactive_transaction()
            self._transaction_session = None

    def run_transactions(self, shell_transactions):
        for shell_transaction in shell_transactions:
            self.handle_transaction(shell_transaction)

    def process_input(self, input_line):
        openTx = self._is_interactive_transaction
        statements = [statement.strip() for statement in input_line.strip().strip(";").split(';')]
        shell_transactions = []
        shell_transaction = None
        for statement in statements:
            if statement.lower() == "start":
                if openTx:
                    raise QuerySyntaxError("Transaction needs to be committed or aborted before starting new one")
                openTx = True
                shell_transaction = ShellTransaction([], None)
            elif statement.lower() == "commit":
                if openTx is False:
                    raise QuerySyntaxError("Commit used before transaction was started")
                if shell_transaction is None:
                    shell_transaction = ShellTransaction([], None)
                shell_transaction.set_outcome(Outcome.COMMIT)
                openTx = False
                shell_transactions.append(shell_transaction)
                shell_transaction = None
            elif statement.lower() == "abort":
                if openTx is False:
                    raise QuerySyntaxError("Abort used before transaction was started")
                if shell_transaction is None:
                    shell_transaction = ShellTransaction([], None)
                shell_transaction.set_outcome(Outcome.ABORT)
                openTx = False
                shell_transactions.append(shell_transaction)
                shell_transaction = None
            elif statement.lower().strip() == "":
                continue
            else:
                if openTx is False:
                    raise QuerySyntaxError("A PartiQL statement was used before a transaction was started")
                if shell_transaction is None:
                    shell_transaction = ShellTransaction([], None)
                shell_transaction.add_query(statement)
        if shell_transaction is not None:
            shell_transactions.append(shell_transaction)
        return shell_transactions

    def handle_transaction(self, shell_transaction):
        if self._transaction_session is None:
            self._transaction_session = self._driver.get_session()

        if self._driver_transaction is None:
            self._driver_transaction = self._transaction_session.start_transaction()
        transaction_id = self._driver_transaction.transaction_id

        if shell_transaction.is_start():
            self.open_interactive_transaction(self._driver_transaction)
            self._transaction_session = self._transaction_session
            return
        elif shell_transaction.is_open():
            self.open_interactive_transaction(self._driver_transaction)
            self._transaction_session = self._transaction_session

        try:
            shell_transaction.run_transaction(self._driver_transaction)
        except ClientError as ce:
            shell_transaction.set_outcome(Outcome.ABORT)
            logging.info("Transaction with transaction_id {} aborted".format(transaction_id))
            shell_transaction.execute_outcome(self._driver_transaction)
            raise ce
        shell_transaction.execute_outcome(self._driver_transaction)

        if shell_transaction.get_outcome() is not None:
            if shell_transaction.get_outcome() == Outcome.ABORT:
                logging.info("Transaction with transaction_id {} aborted".format(transaction_id))
            elif shell_transaction.get_outcome() == Outcome.COMMIT:
                logging.info("Transaction with transaction_id {} committed".format(transaction_id))

        if shell_transaction.get_outcome() is not None:
            self.close_interactive_transaction()
            self._transaction_session.close()
            self._transaction_session = None

    def close_interactive_transaction(self):
        self._driver_transaction = None
        self.prompt = 'qldbshell > '
        self._is_interactive_transaction = False

    def open_interactive_transaction(self, driver_transaction):
        self._driver_transaction = driver_transaction
        self.prompt = 'qldbshell(tx: {}) > '.format(self._driver_transaction.transaction_id)
        self._is_interactive_transaction = True
