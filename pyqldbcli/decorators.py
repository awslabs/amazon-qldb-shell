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

import functools
import logging
import time


def time_this(func):
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        start = time.perf_counter()
        func(*args, **kwargs)
        end = time.perf_counter()
        run_time = end - start
        logging.info(f'({run_time:.4f}s)')
    return wrapper

def single_noun_command(func):
    @functools.wraps(func)
    def wrapper(self, line):
        if len(list(filter(lambda x: x != '', line.split(' ')))) != 1:
            logging.warning(f'This command requires exactly 1 argument. See help <command> for more information. Recieved {line.split(" ")}')
        else:
            func(self, line)
    return wrapper

def zero_noun_command(func):
    @functools.wraps(func)
    def wrapper(self, line):
        if len(list(filter(lambda x: x != '', line.split(' ')))) != 0:
            logging.warning(f'This command requires exactly 0 arguments. See help <command> for more information. Recieved {line.split(" ")}')
        else:
            func(self, line)
    return wrapper
