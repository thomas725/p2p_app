# p2p_app

this is my p2p playground, as of now it's not much more than a modified copy of the libp2p chat example with diesel sqlite

## todo

* build for and run on various embedded Linux devices like OpenWRT
* use Ratatiu crate to start building a better user interface
* store contacts in database + allow to pick & change usernames + autogenerate them

## done

* research how to make diesel create the tables during runtime = embed & run the migration scripts depending on current database state
