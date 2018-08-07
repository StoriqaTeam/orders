use models::*;

use stq_acl::*;
use stq_db::repo::*;

const TABLE: &str = "orders";

pub trait OrderRepo: DbRepo<DbOrder, OrderInserter, OrderFilter, OrderUpdater, RepoError> {}

pub type OrderRepoImpl = DbRepoImpl<DbOrder, OrderInserter, OrderFilter, OrderUpdater>;
impl OrderRepo for OrderRepoImpl {}

type Repo = OrderRepoImpl;

pub fn make_su_repo() -> Repo {
    Repo::new(TABLE)
}

type AclContext = (DbOrder, Action);

fn check_acl(login: UserLogin, (entry, action): &mut AclContext) -> bool {
    use self::RepoLogin::*;
    use models::UserRole::*;

    if let User { caller_roles, caller_id } = login {
        for role_entry in caller_roles {
            match role_entry.role {
                Superadmin => {
                    return true;
                }
                StoreManager(managed_store) => {
                    if managed_store == entry.0.store {
                        return *action != Action::Delete;
                    }
                }
            }
        }

        if caller_id == entry.0.customer {
            return *action != Action::Delete;
        }
    }

    false
}

pub fn make_repo(login: UserLogin) -> Repo {
    make_su_repo().with_afterop_acl_engine(InfallibleSyncACLFn(move |ctx: &mut AclContext| check_acl(login.clone(), ctx)))
}
