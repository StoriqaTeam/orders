use models::*;

use either;
use futures::future;
use futures::prelude::*;
use std::rc::Rc;
use stq_db::repo::*;
use stq_db::statement::*;
use stq_types::*;

use acl::OrdersAcl;

const USER_TABLE: &str = "cart_items_user";
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

        let CartItemInserter { strategy, data } = inserter;

        match split_cart_item(data) {
            Left(data) => Box::new(
                self.user
                    .insert(conn, CartItemUserInserter { strategy, data })
                    .map(|(v, conn)| (v.into_iter().map(From::from).collect(), conn)),
            ),
            Right(data) => Box::new(
                self.session
                    .insert(conn, CartItemSessionInserter { strategy, data })
                    .map(|(v, conn)| (v.into_iter().map(From::from).collect(), conn)),
            ),
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
                Anonymous(session_id) => Box::new(
                    self.session
                        .select_full(
                            conn,
                            CartItemSessionFilter {
                                meta_filter,
                                session_id: Some(session_id),
                            },
                            limit,
                            op,
                        ).map(|(v, conn)| (v.into_iter().map(From::from).collect(), conn)),
                ),
                User(user_id) => Box::new(
                    self.user
                        .select_full(
                            conn,
                            CartItemUserFilter {
                                meta_filter,
                                user_id: Some(user_id),
                            },
                            limit,
                            op,
                        ).map(|(v, conn)| (v.into_iter().map(From::from).collect(), conn)),
                ),
            },
            None => {
                let user = self.user.clone();
                let session = self.session.clone();

                Box::new(
                    future::ok((vec![], conn))
                        .and_then({
                            let meta_filter = meta_filter.clone();
                            move |(mut out, conn)| {
                                user.select_full(conn, meta_filter.into(), limit, op).map(move |(v, conn)| {
                                    for item in v {
                                        out.push(item.into());
                                    }

                                    (out, conn)
                                })
                            }
                        }).and_then({
                            let meta_filter = meta_filter.clone();
                            move |(mut out, conn)| {
                                session.select_full(conn, meta_filter.into(), limit, op).map(move |(v, conn)| {
                                    for item in v {
                                        out.push(item.into());
                                    }

                                    (out, conn)
                                })
                            }
                        }),
                )
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
                Anonymous(session_id) => Box::new(
                    self.session
                        .update(
                            conn,
                            CartItemSessionUpdater {
                                data,
                                filter: CartItemSessionFilter {
                                    session_id: Some(session_id),
                                    meta_filter: filter.meta_filter,
                                },
                            },
                        ).map(|(v, conn)| (v.into_iter().map(From::from).collect(), conn)),
                ),
                User(user_id) => Box::new(
                    self.user
                        .update(
                            conn,
                            CartItemUserUpdater {
                                data,
                                filter: CartItemUserFilter {
                                    user_id: Some(user_id),
                                    meta_filter: filter.meta_filter,
                                },
                            },
                        ).map(|(v, conn)| (v.into_iter().map(From::from).collect(), conn)),
                ),
            },
            None => {
                let user = self.user.clone();
                let session = self.session.clone();

                Box::new(
                    future::ok((vec![], conn))
                        .and_then({
                            let data = data.clone();
                            let meta_filter = filter.meta_filter.clone();
                            move |(mut out, conn)| {
                                user.update(
                                    conn,
                                    CartItemUserUpdater {
                                        data,
                                        filter: CartItemUserFilter {
                                            user_id: None,
                                            meta_filter,
                                        },
                                    },
                                ).map(move |(v, conn)| {
                                    for item in v {
                                        out.push(item.into());
                                    }

                                    (out, conn)
                                })
                            }
                        }).and_then({
                            let data = data.clone();
                            let meta_filter = filter.meta_filter.clone();
                            move |(mut out, conn)| {
                                session
                                    .update(
                                        conn,
                                        CartItemSessionUpdater {
                                            data,
                                            filter: CartItemSessionFilter {
                                                session_id: None,
                                                meta_filter,
                                            },
                                        },
                                    ).map(move |(v, conn)| {
                                        for item in v {
                                            out.push(item.into());
                                        }

                                        (out, conn)
                                    })
                            }
                        }),
                )
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
                Anonymous(session_id) => Box::new(
                    self.session
                        .delete(
                            conn,
                            CartItemSessionFilter {
                                meta_filter,
                                session_id: Some(session_id),
                            },
                        ).map(|(v, conn)| (v.into_iter().map(From::from).collect(), conn)),
                ),
                User(user_id) => Box::new(
                    self.user
                        .delete(
                            conn,
                            CartItemUserFilter {
                                meta_filter,
                                user_id: Some(user_id),
                            },
                        ).map(|(v, conn)| (v.into_iter().map(From::from).collect(), conn)),
                ),
            },
            None => {
                let user = self.user.clone();
                let session = self.session.clone();

                Box::new(
                    future::ok((vec![], conn))
                        .and_then({
                            let meta_filter = meta_filter.clone();
                            move |(mut out, conn)| {
                                user.delete(conn, meta_filter.into()).map(move |(v, conn)| {
                                    for item in v {
                                        out.push(item.into());
                                    }

                                    (out, conn)
                                })
                            }
                        }).and_then({
                            let meta_filter = meta_filter.clone();
                            move |(mut out, conn)| {
                                session.delete(conn, meta_filter.into()).map(move |(v, conn)| {
                                    for item in v {
                                        out.push(item.into());
                                    }

                                    (out, conn)
                                })
                            }
                        }),
                )
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

fn check_acl(login: UserLogin, (entry, _action): &mut AclContext) -> bool {
    use self::RepoLogin::*;
    use models::UserRole::*;

    if let User { caller_roles, caller_id } = login {
        for user_entry in caller_roles {
            if user_entry.role == Superadmin {
                // Superadmins can access in all cases.
                return true;
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
        user: match Rc::try_unwrap(user) {
            Ok(v) => v
                .with_afterop_acl_engine(OrdersAcl(move |ctx: &mut AclContext| check_acl(login.clone(), ctx)))
                .into(),
            Err(_) => unreachable!(),
        },
        session,
    }
}
