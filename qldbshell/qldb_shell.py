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

from amazon.ion.simpleion import dumps
from botocore.exceptions import ClientError, EndpointConnectionError, NoCredentialsError
from pyqldb.cursor.buffered_cursor import BufferedCursor
from pyqldb.errors import SessionPoolEmptyError

from qldbshell.decorators import (time_this, zero_noun_command)

from . import version
from .errors import NoCredentialError


def print_result(cursor: BufferedCursor):
    results = list(map(lambda x: dumps(x, binary=False,
                                                  indent=' ', omit_version_marker=True), cursor))
    logging.info("\n" + str(',\n').join(results))


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
        self._in_session = True


    prompt = 'qldbshell > '

    intro = dedent(f"""\
        Welcome to the Amazon QLDB Shell version {version}

        All future commands will be interpreted as PartiQL statements until the 'exit' command is issued.
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
        # If currently in a session, treat unrecognized input as PartiQL. Else, do nothing.
        if self._in_session:
            session = self._driver.get_session()
            try:
                print_result(session.execute_lambda(
                    lambda x: x.execute_statement(line)))
            except ClientError as e:
                logging.warning(f'Error while executing query: {e}')
            finally:
                session.close()
        else:
            self.do_help('')
