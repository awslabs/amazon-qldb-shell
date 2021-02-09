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
from amazon.ion.simpleion import dumps
from pyqldb.cursor.buffered_cursor import BufferedCursor


def print_result(cursor: BufferedCursor, show_stats):
    results = list(map(lambda x: dumps(x, binary=False,
                                       indent=' ', omit_version_marker=True), cursor))
    logging.info("\n" + str(',\n').join(results))
    if show_stats:
        consumed_ios = cursor.get_consumed_ios()
        timing_info = cursor.get_timing_information()
        read_ios = 'unavailable'
        processing_time = 'unavailable'
        if consumed_ios:
            read_ios = consumed_ios.get('ReadIOs')
        if timing_info:
            processing_time = '{}ms'.format(timing_info.get('ProcessingTimeMilliseconds'))
        logging.info('Read IOs: {}, Server-side latency: {}'.format(read_ios, processing_time))


reserved_words = ['abort',
                  'absolute',
                  'action',
                  'add',
                  'all',
                  'allocate',
                  'alter',
                  'and',
                  'any',
                  'are',
                  'as',
                  'asc',
                  'assertion',
                  'at',
                  'authorization',
                  'avg',
                  'bag',
                  'begin',
                  'between',
                  'bit',
                  'bit_length',
                  'blob',
                  'bool',
                  'boolean',
                  'both',
                  'by',
                  'cascade',
                  'cascaded',
                  'case',
                  'cast',
                  'catalog',
                  'char',
                  'character',
                  'character_length',
                  'char_length',
                  'check',
                  'clob',
                  'close',
                  'coalesce',
                  'collate',
                  'collation',
                  'column',
                  'commit',
                  'connect',
                  'connection',
                  'constraint',
                  'constraints',
                  'continue',
                  'convert',
                  'corresponding',
                  'count',
                  'create',
                  'cross',
                  'current',
                  'current_date',
                  'current_time',
                  'current_timestamp',
                  'current_user',
                  'cursor',
                  'date',
                  'date_add',
                  'date_diff',
                  'day',
                  'deallocate',
                  'dec',
                  'decimal',
                  'declare',
                  'default',
                  'deferrable',
                  'deferred',
                  'delete',
                  'desc',
                  'describe',
                  'descriptor',
                  'diagnostics',
                  'disconnect',
                  'distinct',
                  'domain',
                  'double',
                  'drop',
                  'else',
                  'end',
                  'end-exec',
                  'escape',
                  'except',
                  'exception',
                  'exec',
                  'execute',
                  'exists',
                  'exit',
                  'external',
                  'extract',
                  'false',
                  'fetch',
                  'first',
                  'float',
                  'for',
                  'foreign',
                  'found',
                  'from',
                  'full',
                  'get',
                  'global',
                  'go',
                  'goto',
                  'grant',
                  'group',
                  'having',
                  'hour',
                  'identity',
                  'immediate',
                  'in',
                  'index',
                  'indicator',
                  'initially',
                  'inner',
                  'input',
                  'insensitive',
                  'insert',
                  'int',
                  'integer',
                  'intersect',
                  'interval',
                  'into',
                  'is',
                  'isolation',
                  'join',
                  'key',
                  'language',
                  'last',
                  'leading',
                  'left',
                  'level',
                  'like',
                  'limit',
                  'list',
                  'local',
                  'lower',
                  'match',
                  'max',
                  'min',
                  'minute',
                  'missing',
                  'module',
                  'month',
                  'names',
                  'national',
                  'natural',
                  'nchar',
                  'next',
                  'no',
                  'not',
                  'null',
                  'nullif',
                  'numeric',
                  'octet_length',
                  'of',
                  'on',
                  'only',
                  'open',
                  'option',
                  'or',
                  'order',
                  'outer',
                  'output',
                  'overlaps',
                  'pad',
                  'partial',
                  'pivot',
                  'position',
                  'precision',
                  'prepare',
                  'preserve',
                  'primary',
                  'prior',
                  'privileges',
                  'procedure',
                  'public',
                  'quit',
                  'read',
                  'real',
                  'references',
                  'relative',
                  'remove',
                  'restrict',
                  'revoke',
                  'right',
                  'rollback',
                  'rows',
                  'schema',
                  'scroll',
                  'second',
                  'section',
                  'select',
                  'session',
                  'session_user',
                  'set',
                  'sexp',
                  'size',
                  'smallint',
                  'some',
                  'space',
                  'sql',
                  'sqlcode',
                  'sqlerror',
                  'sqlstate',
                  'start',
                  'string',
                  'struct',
                  'substring',
                  'sum',
                  'symbol',
                  'system_user',
                  'table',
                  'temporary',
                  'then',
                  'time',
                  'timestamp',
                  'timezone_hour',
                  'timezone_minute',
                  'to',
                  'to_string',
                  'to_timestamp',
                  'trailing',
                  'transaction',
                  'translate',
                  'translation',
                  'trim',
                  'true',
                  'tuple',
                  'txid',
                  'undrop',
                  'union',
                  'unique',
                  'unknown',
                  'unpivot',
                  'update',
                  'upper',
                  'usage',
                  'user',
                  'using',
                  'utcnow',
                  'value',
                  'values',
                  'varchar',
                  'varying',
                  'view',
                  'when',
                  'whenever',
                  'where',
                  'with',
                  'work',
                  'write',
                  'year',
                  'zone']
