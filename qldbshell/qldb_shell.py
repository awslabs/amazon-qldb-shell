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

import boto3
from amazon.ion.simpleion import dumps
from botocore.exceptions import ClientError, EndpointConnectionError
from pyqldb.cursor.buffered_cursor import BufferedCursor
from pyqldb.driver.pooled_qldb_driver import PooledQldbDriver

from errors import IllegalStateError
from qldbshell.decorators import (time_this, zero_noun_command)

from . import version


def print_result(cursor: BufferedCursor):
    results = str().join(list(map(lambda x: dumps(x, binary=False,
                                                  indent=' ', omit_version_marker=True) + ",\n", cursor)))
    logging.info("\n" + results)


class QldbShell(cmd.Cmd):

    def __init__(self, profile=None, qldb_endpoint=None, qldb_session_endpoint=None, region=None, ledger=None):
        super(QldbShell, self).__init__()
        self._boto3_session = boto3.Session(
            region_name=region, profile_name=profile)
        self._qldb = self._boto3_session.client(
            'qldb', endpoint_url=qldb_endpoint)
        self._qldb_session_endpoint = qldb_session_endpoint
        self._in_session = False
        if region is None:
            raise IllegalStateError("Region must be specified. Example: us-east-1")
        if ledger is None:
            raise IllegalStateError("Ledger must be specified")
        self._in_session = True
        self._driver = PooledQldbDriver(
            ledger, endpoint_url=self._qldb_session_endpoint, boto3_session=self._boto3_session)


    prompt = 'qldbshell > '

    intro = dedent(f"""\
        Welcome to the Amazon QLDB Shell version {version}

        All future commands will be interpreted as PartiQL statements until the 'quit' command is issued.
    """)

    def onecmd(self, line):
        try:
            return super().onecmd(line)
        except EndpointConnectionError as e:
            logging.fatal(f'Unable to connect to an endpoint. Please check CLI configuration. {e}')
            self.quit_shell()
        except ClientError as e:
            logging.error(f'Error encountered: {e}')
            return False # don't stop

    def do_EOF(self, line):
        'Exits the CLI; equivalent to calling quit: EOF'
        self.quit_shell()

    def quit_shell(self):
        logging.info("Exiting qldb shell.")
        exit(0)

    @zero_noun_command
    def do_quit(self, line):
        'Exit the qldb shell: quit'
        self.quit_shell()

    @time_this
    def default(self, line):
        'If currently in a session, treat unrecognized input as PartiQL. Else, do nothing.'
        if self._in_session:
            session = self._driver.get_session()
            try:
                logging.info(print_result(session.execute_lambda(
                    lambda x: x.execute_statement(line))))
            except ClientError as e:
                logging.warning(f'Error while executing query: {e}')
        else:
            self.do_help('')
