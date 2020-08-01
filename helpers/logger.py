import logging
import os
import traceback

from datahandlers.local_datahandler import get_data_by_filename as get_data
from static import static


# Create or return an existing logger for the specified file
def get_log(sys, level=static.DEBUG_LEVEL):
    # Get the filename of the OG page
    filename = get_filename(sys)
    # If the logger already exists, just return it
    if logging.getLogger().hasHandlers():
        log = logging.getLogger(filename)
        log.info(f"Found logger '{log.name}' with level '{logging.getLevelName(log.level)}'")
    # Otherwise, make a new one
    else:
        logging_path = get_logging_path(f"{filename}_{static.VERSION_NUMBER}")
        log = create_logger(filename, level, logging_path)
        log.info(f"Created new logger '{log.name}' with level '{logging.getLevelName(log.level)}'")
    return log


# Get the filename of sys, trim extensions and directories
def get_filename(sys):
    try:
        filename = os.path.splitext(os.path.basename(os.path.realpath(sys.argv[0]) if sys.argv[0] else None))[0]
        return filename
    except (ValueError, Exception):
        logging.error(traceback.format_exc())
        logging.warning("Couldn't get the filename of sys")
        return None


# Get the path of the logging file
def get_logging_path(filename):
    try:
        logging_path = get_data(filename, is_log=True, return_path_only=True)
        return logging_path
    except (ValueError, Exception):
        logging.error(traceback.format_exc())
        logging.warning("Couldn't get logging path")
        return None


# Create a new logger
def create_logger(filepath, level, logging_path="general_log"):
    logging.basicConfig()
    logger = logging.getLogger(filepath)
    logger.setLevel(level)
    fhan = logging.FileHandler(logging_path)
    fhan.setLevel(level)
    logger.addHandler(fhan)
    formatter = logging.Formatter("%(asctime)s - %(levelname)s: %(message)s", "%d-%m-%Y %H:%M:%S")
    fhan.setFormatter(formatter)
    return logger


def get_log_by_filename(filename):
    return logging.Logger(filename)
