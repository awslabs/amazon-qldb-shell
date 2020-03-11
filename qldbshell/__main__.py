#!/usr/bin/env python

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

import argparse
import logging
import sys

from botocore.exceptions import ClientError, EndpointConnectionError

from .qldb_shell import QldbShell


def main():
    parser = argparse.ArgumentParser(
        description="A shell wrapper around `pyqldb`, the Amazon QLDB Driver for Python.",
        epilog="As an alternative to the commandline, params can be placed in a file, one per line, and specified on the commandline like '%(prog)s @params.conf'.",
        fromfile_prefix_chars='@')
    required_named = parser.add_argument_group('required named arguments')
    parser.add_argument(
        "-v",
        "--verbose",
        help="increase output verbosity",
        action="store_true")
    parser.add_argument(
        "-s",
        "--qldb-session-endpoint",
        help="Endpoint to use for the qldb-session API",
        action="store",
        dest="qldb_session_endpoint"
    )
    parser.add_argument(
        "-r",
        "--region",
        help="AWS Region to use for credentials and/or endpoint formation, e.g. us-east-1",
        action="store"
    )
    parser.add_argument(
        "-p",
        "--profile",
        help="Name of a profile speficified in aws credentials setup whose credentials we should use",
        action="store",
    )
    required_named.add_argument(
        "-l",
        "--ledger",
        help="Name of a ledger to connect to",
        action="store",
        required=True
    )
    args = parser.parse_args()

  # Setup logging
    if args.verbose:
        loglevel = logging.DEBUG
    else:
        loglevel = logging.INFO
    logging.basicConfig(format="%(levelname)s: %(message)s", level=loglevel)
    cli = QldbShell(args.profile,
                    qldb_session_endpoint=args.qldb_session_endpoint, region=args.region, ledger=args.ledger)
    cli.cmdloop()


# Standard boilerplate to call the main() function to begin
# the program.
if __name__ == '__main__':
    main()
