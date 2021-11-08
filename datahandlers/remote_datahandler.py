import sys
import traceback
from datetime import datetime
from typing import Optional

import sshtunnel
from sqlalchemy import create_engine
from sqlalchemy.exc import SQLAlchemyError, OperationalError
from sqlalchemy.orm import sessionmaker

from helpers import logger
from helpers.utils import check_if_ide
from models.entry import Entry
from models.link import Link
from models.table import Table
from static import static

log = logger.get_log(sys)


# Get and return a new SQL session, use SSH if necessary
def get_engine_session():
    try:
        if check_if_ide() is True:
            engine = get_engine(ssh=True)
        else:
            engine = get_engine(ssh=False)

        return setup_session(engine)

    except (SQLAlchemyError, Exception):
        log.error(traceback.format_exc())
        log.warning("Couldn't execute statement!")


# Create and return an engine for SQL transactions
def get_engine(ssh):
    # If run with SSH, create and start a SSH tunnel and generate a URI with the tunnel
    if ssh:
        log.info("Setting up SSH tunnel")
        sshtunnel.SSH_TIMEOUT = static.SSH_SSH_TIMEOUT
        sshtunnel.TUNNEL_TIMEOUT = static.SSH_TUNNEL_TIMEOUT

        tunnel = sshtunnel.SSHTunnelForwarder(
            static.SSH_HOSTNAME, ssh_username=static.SSH_USERNAME, ssh_password=static.SSH_PASSWORD,
            remote_bind_address=(static.DB_HOSTNAME, static.DB_SERVER_PORT)
        )

        tunnel.start()
        SQLALCHEMY_DATABASE_URI = "mysql+mysqlconnector://{username}:{password}@" \
                                  "{db_local_port}:{tunnel_local_bind_port}/{databasename}".format(
                                    username=static.DB_USERNAME,
                                    password=static.DB_PASSWORD,
                                    hostname=static.DB_HOSTNAME,
                                    db_local_port=static.DB_LOCAL_PORT,
                                    tunnel_local_bind_port=tunnel.local_bind_port,
                                    databasename=static.DB_DATABASENAME)

    # If run without SSH, only generate a plain URI
    else:
        SQLALCHEMY_DATABASE_URI = "mysql+mysqlconnector://{username}:{password}@" \
                                  "{hostname}/{databasename}".format(
                                    username=static.DB_USERNAME,
                                    password=static.DB_PASSWORD,
                                    hostname=static.DB_HOSTNAME,
                                    databasename=static.DB_DATABASENAME)

    # Try to create an engine that is refreshed every x seconds
    try:
        engine = create_engine(SQLALCHEMY_DATABASE_URI, pool_recycle=280)
        return engine

    except (SQLAlchemyError, Exception):
        log.error(traceback.format_exc())
        log.warning("Couldn't setup engine!")
        return None


# Create a new session and bind the engine
def setup_session(engine):
    try:
        Session = sessionmaker(bind=engine)
        session = Session()
        return session
    except (SQLAlchemyError, Exception):
        log.error(traceback.format_exc())
        log.warning("Couldn't setup session!")


# Get the data
def get_data(session, limit, offset, order_descending, entry_id=None, entry_type=None, handled_utc=None,
             original_url=None, canonical_url=None, canonical_type=None, note=None) -> [Entry]:
    log.info(f"Getting data of type {entry_type} from {Table.__tablename__},"
             f" limit = {limit}, order_descending = {order_descending}")

    # Store the values in a dict
    filter_options = {
        Table.entry_id: entry_id,
        Table.entry_type: entry_type,
        Table.handled_utc: handled_utc,
        Table.original_url: original_url,
        Table.canonical_url: canonical_url,
        Table.canonical_type: canonical_type,
        Table.note: note
    }

    # Create a new query
    q = session.query(Table)

    for attr, value in filter_options.items():
        log.debug(f"attr= {attr}, value={value}")

        # If true, select not NULL
        if value is True:
            q = q.filter(attr.isnot(None))

        # If false, select NULL
        elif value is False:
            q = q.filter(attr == None)

        # If anything else, select X
        elif value is not None:
            q = q.filter(attr == value)

    # Sort descending (returns most recent rows)
    if order_descending:
        q = q.order_by(Table.entry_id.desc())

    if offset:
        q = q.offset(offset)

    # Set a limit
    q = q.limit(limit)
    log.info(q)
    log.info(f"Received data, rows: {q.count()}")

    # Generate entry instance for each returned row, add these to a list
    entries = []
    for entry in q:
        entries.append(Entry(
            entry_id=entry.entry_id,
            entry_type=entry.entry_type,
            handled_utc=entry.handled_utc,
            original_url=entry.original_url,
            canonical_url=entry.canonical_url,
            canonical_type=entry.canonical_type,
            note=entry.note
        ))

    log.info("Entry instance(s) generated")

    return entries


def get_entry_by_original_url(original_url, session=get_engine_session()) -> Optional[Entry]:
    try:
        q = session.query(Table)
        q = q.filter(Table.original_url == original_url)
        q = q.filter(Table.canonical_url is not None)
        q = q.order_by(Table.entry_id.desc())
        q = q.first()
        if q:
            return q

    except (SQLAlchemyError, OperationalError, Exception):
        log.error(traceback.format_exc())
        log.warning(f"Failed to get entry by original url {original_url}")

    return None


def add_data(session, entry_type=None, handled_utc=None, original_url=None,
             canonical_url=None, canonical_type=None, note=None):
    try:
        log.info(f"Sending data to database..")

        new_entry = Table(
            entry_type=entry_type,
            handled_utc=handled_utc,
            original_url=original_url,
            canonical_url=canonical_url,
            canonical_type=canonical_type,
            note=note
        )

        session.add(new_entry)
        session.commit()
        log.info("Entry send to database")

    except (SQLAlchemyError, Exception):
        log.error(traceback.format_exc())
        log.warning("Couldn't send entry to database!")


# If save_to_database is enabled, make a new entry for every URL
def save_entry(save_to_database, entry_type, links: [Link]):
    if save_to_database:
        for link in links:
            if link.origin and link.origin.is_amp:
                add_data(session=get_engine_session(),
                         entry_type=entry_type,
                         handled_utc=datetime.now().strftime('%Y-%m-%d %H:%M:%S'),
                         original_url=link.origin.url,
                         canonical_url=link.canonical.url if link.canonical else None,
                         canonical_type=link.canonical.type if link.canonical else None)
