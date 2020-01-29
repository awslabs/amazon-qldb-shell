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

from pyqldbcli.decorators import (single_noun_command, time_this,
                                  zero_noun_command)

from . import version


def print_result(cursor: BufferedCursor):
    results = str().join(list(map(lambda x: dumps(x, binary=False,
                                                  indent=' ', omit_version_marker=True) + ",\n", cursor)))
    logging.info("\n" + results)


class QldbCli(cmd.Cmd):

    def __init__(self, profile=None, qldb_endpoint=None, qldb_session_endpoint=None, region=None, ledger=None):
        super(QldbCli, self).__init__()
        self._boto3_session = boto3.Session(
            region_name=region, profile_name=profile)
        self._qldb = self._boto3_session.client(
            'qldb', endpoint_url=qldb_endpoint)
        self._qldb_session_endpoint = qldb_session_endpoint
        self._in_session = False
        if ledger:
            self._in_session = True
            self._driver = PooledQldbDriver(
                ledger, endpoint_url=self._qldb_session_endpoint, boto3_session=self._boto3_session)

    prompt = 'pyqldbcli > '

    intro = dedent(f"""\
        Welcome to the Amazon QLDB Python CLI version {version}

        Basic Amazon QLDB control plane operations are supported via commands like 'create' or 'delete'.

        To transact with a ledger via PartiQL, use the 'connect' command. Once connected, all future commands
        will be interpreted as PartiQL statements until the 'disconnect' command is issued.

        For more information, type 'help'.
    """)

    def onecmd(self, line):
        try:
            return super().onecmd(line)
        except EndpointConnectionError as e:
            logging.fatal(f'Unable to connect to an endpoint. Please check CLI configuration. {e}')
            self.quit_cli()
        except ClientError as e:
            logging.error(f'Error encountered: {e}')
            return False # don't stop

    def do_EOF(self, line):
        'Exits the CLI; equivalent to calling quit: EOF'
        self.quit_cli()

    def list_ledgers(self):
        return list(map(lambda x: x['Name'], self._qldb.list_ledgers()['Ledgers']))

    def quit_cli(self):
        logging.info("Exiting pyqldb CLI.")
        exit(0)

    @zero_noun_command
    def do_quit(self, line):
        'Exit the pyqldb CLI: quit'
        self.quit_cli()

    @time_this
    @zero_noun_command
    def do_list(self, line):
        'List all available ledgers: list'
        logging.info(f'Ledgers: {self.list_ledgers()}')

    @time_this
    @single_noun_command
    def do_delete(self, line):
        'Delete the specified ledger. delete LEDGER'
        logging.info(f'Deleting ledger {line}.')
        self._qldb.delete_ledger(Name=line)
        logging.info(f'Deleted ledger {line}.')

    @time_this
    @single_noun_command
    def do_create(self, line):
        'Create the specified ledger: create LEDGER'
        logging.info(f'Creating ledger {line}.')
        self._qldb.create_ledger(
            Name=line, PermissionsMode='ALLOW_ALL', DeletionProtection=False)
        logging.info(f'Created ledger {line}.')

    @time_this
    @single_noun_command
    def do_describe(self, line):
        'Describe the specified ledger: describe LEDGER'
        logging.info(f'Describing ledger {line}.')
        response = self._qldb.describe_ledger(Name=line)
        del response['ResponseMetadata']
        logging.info(response)

    @time_this
    @single_noun_command
    def do_connect(self, line):
        'Establish a connection with the specified ledger, enabling PartiQL execution: connect LEDGER'
        if self._in_session:
            self.do_disconnect('')
        logging.info(f'Starting session with ledger {line}.')
        ledgers = self.list_ledgers()
        if line in ledgers:
            logging.info(f'Ledger {line} exists.')
            logging.info(f'Attempting to connect to ledger {line}')
            self._driver = PooledQldbDriver(
                line, endpoint_url=self._qldb_session_endpoint, boto3_session=self._boto3_session)
            self._driver.get_session()
            self._in_session = True
            logging.info(
                f'Ready to transact with ledger {line}. Any subsequent unrecognized commands will be treated as PartiQL queries.')
        else:
            logging.error(f'No ledger found with name {line}')

    @time_this
    @zero_noun_command
    def do_disconnect(self, line):
        'Exit the current database session: disconnect'
        logging.info("Closing current session.")
        self._in_session = False
        self._session = None

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
