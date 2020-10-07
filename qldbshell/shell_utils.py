from pyqldb.cursor.buffered_cursor import BufferedCursor
import logging
from amazon.ion.simpleion import dumps


def print_result(cursor: BufferedCursor):
    results = list(map(lambda x: dumps(x, binary=False,
                                        indent=' ', omit_version_marker=True), cursor))
    logging.info("\n" + str(',\n').join(results))
