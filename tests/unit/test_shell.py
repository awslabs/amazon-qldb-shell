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
from botocore.exceptions import EndpointConnectionError, ClientError, NoCredentialsError
from pyqldb.driver.pooled_qldb_driver import PooledQldbDriver
from pyqldb.errors import SessionPoolEmptyError

from qldbshell.errors import NoCredentialError
from qldbshell.qldb_shell import QldbShell
from unittest import TestCase
from unittest.mock import patch
import builtins

MOCK_MESSAGE = 'message'


class TestQldbShell(TestCase):

    @patch('pyqldb.driver.pooled_qldb_driver.PooledQldbDriver')
    def test_constructor_success(self, mockdriver):
        mock_shell = QldbShell(None, mockdriver)

        assert mock_shell is not None

    @patch('pyqldb.driver.pooled_qldb_driver.PooledQldbDriver')
    def test_constructor_no_credentials_throws_exception(self, mockdriver):
        mockdriver.get_session.side_effect = NoCredentialsError()

        with self.assertRaises(NoCredentialError):
            QldbShell(None, mockdriver)

    @patch('pyqldb.session.pooled_qldb_session.PooledQldbSession')
    @patch('pyqldb.driver.pooled_qldb_driver.PooledQldbDriver')
    def test_default_success(self, mock_driver, mock_pooled_session):
        mock_driver.get_session.return_value = mock_pooled_session

        mock_cli = QldbShell(None, mock_driver)
        mock_cli._in_session = True
        mock_cli._driver = mock_driver

        statement = "select * from another_table"
        mock_cli.default(statement)

        mock_driver.get_session.assert_called()
        assert mock_pooled_session.execute_lambda.call_count == 2
        mock_pooled_session.close.assert_called()

    @patch('pyqldb.session.pooled_qldb_session.PooledQldbSession')
    @patch('pyqldb.driver.pooled_qldb_driver.PooledQldbDriver')
    def test_default_client_error_session_closed(self, mock_driver, mock_pooled_session):
        mock_driver.get_session.return_value = mock_pooled_session

        mock_cli = QldbShell(None, mock_driver)
        mock_cli._in_session = True
        mock_cli._driver = mock_driver

        mock_invalid_session_error_message = {'Error': {'Code': 'InvalidSessionException',
                                                        'Message': MOCK_MESSAGE}}
        mock_pooled_session.execute_lambda.side_effect = ClientError(mock_invalid_session_error_message, MOCK_MESSAGE)

        statement = "select * from another_table"
        mock_cli.default(statement)

        mock_driver.get_session.assert_called()
        mock_pooled_session.close.assert_called()

    @patch('builtins.super')
    @patch('pyqldb.driver.pooled_qldb_driver.PooledQldbDriver')
    def test_onecmd_connection_failure(self,  mock_driver, mock_super):
        statement = "select * from another_table"
        mock_super.onecmd.side_effect = EndpointConnectionError(endpoint_url=None)
        mock_shell = QldbShell(None, mock_driver)

        mock_shell.onecmd(statement)

        self.assertRaises(SystemExit)

    @patch('qldbshell.qldb_shell.QldbShell.do_exit')
    @patch('qldbshell.qldb_shell.QldbShell._strip_text')
    @patch('qldbshell.qldb_shell.QldbShell.onecmd')
    @patch('qldbshell.qldb_shell.PromptSession')
    @patch('pyqldb.driver.pooled_qldb_driver.PooledQldbDriver')
    def test_escape_sequences(self, mock_driver, mock_prompt_session, mock_onecmd, mock_strip_text, mock_do_exit):
        mock_prompt_session.return_value = mock_prompt_session
        mock_prompt_session.prompt.return_value = r'\\'
        mock_strip_text.side_effect = ['', 'quit']
        shell = QldbShell(None, mock_driver)
        shell.cmdloop("test-ledger")
        mock_onecmd.assert_called_with("\\")


