import logging
import os
import sys
from typing import List, Union

from static import static

# Can't use logger.get_log(sys) because of circular import
log = logging.getLogger(os.path.splitext(os.path.basename(os.path.realpath(sys.argv[0]) if sys.argv[0] else None))[0])


# Get the data of file X
def get_data_by_filename(filename, is_log=False, return_path_only=False) -> Union[str, List[str]]:
    path = get_path(filename, is_log)
    if not os.path.isfile(path):
        log.warning(f"{path} could not be found, creating an empty file now")
        open(path, "w+").close()

    if return_path_only:
        log.info("returning path to filename only")
        return path

    with open(path, "r") as f:
        data = f.read().split(",")
        log.info(f"{path} was found.")
        f.close()

    return data


# Get the path to the specified file
def get_path(filename, is_log=False) -> str:
    if is_log:
        directory = static.LOG_FOLDER_NAME
        file_extension = "log"

    else:
        directory = static.DATA_FOLDER_NAME
        file_extension = "txt"

    path = f"{directory}/{filename}.{file_extension}"
    return path


# Add a new item to the specified file
# If 'unique' is true, the file is first checked if it doesn't contain the item_to_add already
def update_local_data(filename, item_to_add, unique=False):
    path = get_path(filename)
    with open(path, "a") as f:
        if unique and item_to_add in f.read().split():
            log.warning(f"Didn't add {item_to_add} to {filename}: not unique")
        else:
            f.write(f",{item_to_add}")
            log.info(f"Added {item_to_add} to {filename}")
            f.close()


# Remove the specified item from the specified file
def remove_local_data(filename, item_to_remove):
    path = get_path(filename)
    with open(path, "a+") as f:
        updated_list = f.read().replace(f",{item_to_remove}", "")
        f.close()
    with open(path, "w") as f:
        f.write(updated_list)
        log.info(f"Removed {item_to_remove} from {filename}")
        f.close()
