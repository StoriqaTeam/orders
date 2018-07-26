use models::*;

use either;
use futures::future;
use futures::prelude::*;
use std::rc::Rc;
use stq_acl::*;
use stq_db::repo::*;
use stq_db::statement::*;
use stq_types::*;

const USER_TABLE: &str = "cart_items";
const SESSION_TABLE: &str = "cart_items_session";

pub trait CartItemUserRepo: DbRepo<CartItemUser, CartItemUserInserter, CartItemUserFilter, CartItemUserUpdater, RepoError> {}
pub type CartItemUserRepoImpl = DbRepoImpl<CartItemUser, CartItemUserInserter, CartItemUserFilter, CartItemUserUpdater>;

pub trait CartItemSessionRepo:
    DbRepo<CartItemSession, CartItemSessionInserter, CartItemSessionFilter, CartItemUpdater<CartItemSessionFilter>, RepoError>
{
}

pub type CartItemSessionRepoImpl =
    DbRepoImpl<CartItemSession, CartItemSessionInserter, CartItemSessionFilter, CartItemUpdater<CartItemSessionFilter>>;

pub struct CartItemRepoImpl {
    user: Rc<CartItemUserRepoImpl>,
    session: Rc<CartItemSessionRepoImpl>,
}

pub trait CartItemRepo: DbRepo<CartItem, CartItemInserter, CartItemFilter, CartItemUpdater<CartItemFilter>, RepoError> {}
impl CartItemRepo for CartItemRepoImpl {}

impl DbRepo<CartItem, CartItemInserter, CartItemFilter, CartItemUpdater<CartItemFilter>, RepoError> for CartItemRepoImpl {}

impl DbRepoInsert<CartItem, CartItemInserter, RepoError> for CartItemRepoImpl {
    fn insert(&self, conn: RepoConnection, inserter: CartItemInserter) -> RepoConnectionFuture<Vec<CartItem>> {
        use self::either::Either::*;
        use self::CartCustomer::*;

        let CartItemInserter { strategy, data } = inserter;

        match split_cart_item(data) {
            Left(data) => self.user.insert(conn, CartItemUserInserter { strategy, data }),
            Right(data) => self.session.insert(conn, CartItemSessionInserter { strategy, data }),
        }
    }
}

impl DbRepoSelect<CartItem, CartItemFilter, RepoError> for CartItemRepoImpl {
    fn select_full(
        &self,
        conn: RepoConnection,
        filter: CartItemFilter,
        limit: Option<i32>,
        op: Option<SelectOperation>,
    ) -> RepoConnectionFuture<Vec<CartItem>> {
        use self::CartCustomer::*;

        let CartItemFilter { meta_filter, customer } = filter;

        match customer {
            Some(customer) => match customer {
                Anonymous(session_id) => self.session.select_full(
                    conn,
                    CartItemSessionFilter {
                        meta_filter,
                        session_id: Some(session_id),
                    },
                    limit,
                    op,
                ),
                User(user_id) => self.user.select_full(
                    conn,
                    CartItemUserFilter {
                        meta_filter,
                        user_id: Some(user_id),
                    },
                    limit,
                    op,
                ),
            },
            None => {
                let user = self.user.clone();
                let session = self.session.clone();

                future::ok((vec![], conn))
                    .and_then({
                        let meta_filter = meta_filter.clone();
                        move |(mut out, conn)| {
                            user.select_full(conn, meta_filter.into(), limit, op).map(move |(v, conn)| {
                                out.append(v);

                                (out, conn)
                            })
                        }
                    })
                    .and_then({
                        let meta_filter = meta_filter.clone();
                        move |(mut out, conn)| {
                            session.select_full(conn, meta_filter.into(), limit, op).map(move |(v, conn)| {
                                out.append(v);

                                (out, conn)
                            })
                        }
                    })
            }
        }
    }
}

impl DbRepoUpdate<CartItem, CartItemUpdater<CartItemFilter>, RepoError> for CartItemRepoImpl {
    fn update(&self, conn: RepoConnection, updater: CartItemUpdater<CartItemFilter>) -> RepoConnectionFuture<Vec<CartItem>> {
        use self::CartCustomer::*;

        let CartItemUpdater { data, filter } = updater;

        match filter.customer {
            Some(customer) => match customer {
                Anonymous(session_id) => self.session.update(
                    conn,
                    CartItemSessionUpdater {
                        data,
                        filter: CartItemSessionFilter {
                            session_id: Some(session_id),
                            meta_filter: filter.meta_filter,
                        },
                    },
                ),
                User(user_id) => self.user.update(
                    conn,
                    CartItemUserUpdater {
                        data,
                        filter: CartItemUserFilter {
                            user_id: Some(user_id),
                            meta_filter: filter.meta_filter,
                        },
                    },
                ),
            },
            None => {
                let user = self.user.clone();
                let session = self.session.clone();

                future::ok((vec![], conn))
                    .and_then({
                        let meta_filter = filter.meta_filter.clone();
                        move |(mut out, conn)| {
                            user.update(conn, meta_filter.into()).map(move |(v, conn)| {
                                out.append(v);

                                (out, conn)
                            })
                        }
                    })
                    .and_then({
                        let meta_filter = filter.meta_filter.clone();
                        move |(mut out, conn)| {
                            session.update(conn, meta_filter.into()).map(move |(v, conn)| {
                                out.append(v);

                                (out, conn)
                            })
                        }
                    })
            }
        }
    }
}

impl DbRepoDelete<CartItem, CartItemFilter, RepoError> for CartItemRepoImpl {
    fn delete(&self, conn: RepoConnection, filter: CartItemFilter) -> RepoConnectionFuture<Vec<CartItem>> {
        use self::CartCustomer::*;

        let CartItemFilter { meta_filter, customer } = filter;

        match customer {
            Some(customer) => match customer {
                Anonymous(session_id) => self.session.delete(
                    conn,
                    CartItemSessionFilter {
                        meta_filter,
                        session_id: Some(session_id),
                    },
                ),
                User(user_id) => self.user.delete(
                    conn,
                    CartItemUserFilter {
                        meta_filter,
                        user_id: Some(user_id),
                    },
                ),
            },
            None => {
                let user = self.user.clone();
                let session = self.session.clone();

                future::ok((vec![], conn))
                    .and_then({
                        let meta_filter = meta_filter.clone();
                        move |(mut out, conn)| {
                            user.delete(conn, meta_filter.into()).map(move |(v, conn)| {
                                out.append(v);

                                (out, conn)
                            })
                        }
                    })
                    .and_then({
                        let meta_filter = meta_filter.clone();
                        move |(mut out, conn)| {
                            session.delete(conn, meta_filter.into()).map(move |(v, conn)| {
                                out.append(v);

                                (out, conn)
                            })
                        }
                    })
            }
        }
    }
}

pub fn make_su_repo() -> CartItemRepoImpl {
    CartItemRepoImpl {
        user: CartItemUserRepoImpl::new(USER_TABLE).into(),
        session: CartItemSessionRepoImpl::new(SESSION_TABLE).into(),
    }
}

type AclContext = (CartItemUser, Action);

fn check_acl(login: UserLogin, (entry, action): &mut AclContext) -> bool {
    use self::RepoLogin::*;
    use models::UserRole::*;

    if let User { caller_roles, caller_id } = login {
        for user_entry in caller_roles {
            match user_entry.role {
                // Superadmins can access in all cases.
                Superadmin => {
                    return true;
                }
                _ => {}
            }
        }

        if caller_id == entry.user_id {
            return true;
        }
    }

    false
}

pub fn make_repo(login: UserLogin) -> CartItemRepoImpl {
    let CartItemRepoImpl { user, session } = make_su_repo();
    CartItemRepoImpl {
        user: user.try_unwrap()
            .unwrap()
            .with_afterop_acl_engine(InfallibleSyncACLFn(move |ctx: &mut AclContext| check_acl(login.clone(), ctx)))
            .into(),
        session: session.into(),
    }
}
