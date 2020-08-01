from sqlalchemy import Column, Integer, Enum, DateTime, VARCHAR
from sqlalchemy.ext.declarative import declarative_base

from models.type import Type

Base = declarative_base()


class Table(Base):
    __tablename__ = 'URLConversions'
    entry_id = Column('entry_id', Integer, primary_key=True, autoincrement=True)
    entry_type = Column('entry_type', Enum(Type))
    handled_utc = Column('handled_utc', DateTime)
    original_url = Column('original_url', VARCHAR(4000))
    canonical_url = Column('canonical_url', VARCHAR(4000))
    note = Column('note', VARCHAR(4000))
