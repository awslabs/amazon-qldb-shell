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

from enum import Enum


class Command(Enum):
    START = 0
    EXECUTE = 1
    COMMIT = 2
    ABORT = 3


class CommandContainer:
    """
    Container for the input and output details of commands during an
    interactive transaction session.
    """

    def __init__(self, command, statement=None, output=None):
        self.command = command
        self.statement = statement
        self._output = output

    @property
    def output(self):
        if isinstance(self._output, Exception):
            raise self._output
        else:
            return self._output
