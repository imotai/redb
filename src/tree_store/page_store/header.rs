use crate::transaction_tracker::TransactionId;
use crate::tree_store::Checksum;
use crate::tree_store::btree_base::BtreeHeader;
use crate::tree_store::page_store::layout::{DatabaseLayout, RegionLayout};
use crate::tree_store::page_store::page_manager::{
    FILE_FORMAT_VERSION1, FILE_FORMAT_VERSION2, FILE_FORMAT_VERSION3, xxh3_checksum,
};
use crate::{DatabaseError, Result, StorageError};
use std::mem::size_of;

// Database layout:
//
// Super-header (header + commit slots)
// The super-header length is rounded up to the nearest full page size
//
// Header (first 64 bytes):
// 9 bytes: magic number
// 1 byte: god byte
// 2 byte: padding
// 4 bytes: page size
// Definition of region
// 4 bytes: region header pages
// 4 bytes: region max data pages
//
// Commit slot 0 (next 128 bytes):
// 1 byte: version
// 1 byte: != 0 if root page is non-null
// 1 byte: != 0 if freed table root page is non-null
// 5 bytes: padding
// 8 bytes: root page
// 16 bytes: root checksum
// 8 bytes: unused: formerly freed table root page
// 16 bytes: unused: formerly freed table root checksum
// 8 bytes: last committed transaction id
// 4 bytes: number of full regions
// 4 bytes: data pages in partial trailing region
// 8 bytes: unused: formerly region tracker page number
// 16 bytes: slot checksum
//
// Commit slot 1 (next 128 bytes):
// Same layout as slot 0

// Inspired by PNG's magic number
pub(super) const MAGICNUMBER: [u8; 9] = [b'r', b'e', b'd', b'b', 0x1A, 0x0A, 0xA9, 0x0D, 0x0A];
const GOD_BYTE_OFFSET: usize = MAGICNUMBER.len();
const PAGE_SIZE_OFFSET: usize = GOD_BYTE_OFFSET + size_of::<u8>() + 2; // +2 for padding
const REGION_HEADER_PAGES_OFFSET: usize = PAGE_SIZE_OFFSET + size_of::<u32>();
const REGION_MAX_DATA_PAGES_OFFSET: usize = REGION_HEADER_PAGES_OFFSET + size_of::<u32>();
const NUM_FULL_REGIONS_OFFSET: usize = REGION_MAX_DATA_PAGES_OFFSET + size_of::<u32>();
const TRAILING_REGION_DATA_PAGES_OFFSET: usize = NUM_FULL_REGIONS_OFFSET + size_of::<u32>();
// Formerly the region tracker page
const _UNUSED3_OFFSET: usize = TRAILING_REGION_DATA_PAGES_OFFSET + size_of::<u32>();
const TRANSACTION_SIZE: usize = 128;
const TRANSACTION_0_OFFSET: usize = 64;
const TRANSACTION_1_OFFSET: usize = TRANSACTION_0_OFFSET + TRANSACTION_SIZE;
pub(super) const DB_HEADER_SIZE: usize = TRANSACTION_1_OFFSET + TRANSACTION_SIZE;

// God byte flags
const PRIMARY_BIT: u8 = 1;
const RECOVERY_REQUIRED: u8 = 2;
const TWO_PHASE_COMMIT: u8 = 4;

// Structure of each commit slot
const VERSION_OFFSET: usize = 0;
const USER_ROOT_NON_NULL_OFFSET: usize = size_of::<u8>();
const SYSTEM_ROOT_NON_NULL_OFFSET: usize = USER_ROOT_NON_NULL_OFFSET + size_of::<u8>();
const _UNUSED_OFFSET: usize = SYSTEM_ROOT_NON_NULL_OFFSET + size_of::<u8>();
const PADDING: usize = 4;

const USER_ROOT_OFFSET: usize = _UNUSED_OFFSET + size_of::<u8>() + PADDING;
const SYSTEM_ROOT_OFFSET: usize = USER_ROOT_OFFSET + BtreeHeader::serialized_size();
const _UNUSED2_OFFSET: usize = SYSTEM_ROOT_OFFSET + BtreeHeader::serialized_size();
const TRANSACTION_ID_OFFSET: usize = _UNUSED2_OFFSET + BtreeHeader::serialized_size();
const TRANSACTION_LAST_FIELD: usize = TRANSACTION_ID_OFFSET + size_of::<u64>();

const SLOT_CHECKSUM_OFFSET: usize = TRANSACTION_SIZE - size_of::<Checksum>();

pub(crate) const PAGE_SIZE: usize = 4096;

fn get_u32(data: &[u8]) -> u32 {
    u32::from_le_bytes(data[..size_of::<u32>()].try_into().unwrap())
}

fn get_u64(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[..size_of::<u64>()].try_into().unwrap())
}

#[derive(Copy, Clone)]
pub(super) struct HeaderRepairInfo {
    pub(super) invalid_magic_number: bool,
    pub(super) primary_corrupted: bool,
    pub(super) secondary_corrupted: bool,
}

#[derive(Clone)]
pub(super) struct DatabaseHeader {
    primary_slot: usize,
    pub(super) recovery_required: bool,
    pub(super) two_phase_commit: bool,
    page_size: u32,
    region_header_pages: u32,
    region_max_data_pages: u32,
    full_regions: u32,
    trailing_partial_region_pages: u32,
    transaction_slots: [TransactionHeader; 2],
}

impl DatabaseHeader {
    pub(super) fn new(layout: DatabaseLayout, transaction_id: TransactionId) -> Self {
        #[allow(clippy::assertions_on_constants)]
        {
            assert!(TRANSACTION_LAST_FIELD <= SLOT_CHECKSUM_OFFSET);
        }

        let slot = TransactionHeader::new(transaction_id);
        Self {
            primary_slot: 0,
            recovery_required: true,
            two_phase_commit: false,
            page_size: layout.full_region_layout().page_size(),
            region_header_pages: layout.full_region_layout().get_header_pages(),
            region_max_data_pages: layout.full_region_layout().num_pages(),
            full_regions: layout.num_full_regions(),
            trailing_partial_region_pages: layout
                .trailing_region_layout()
                .map(|x| x.num_pages())
                .unwrap_or_default(),
            transaction_slots: [slot.clone(), slot],
        }
    }

    pub(super) fn page_size(&self) -> u32 {
        self.page_size
    }

    pub(super) fn layout(&self) -> DatabaseLayout {
        let full_layout = RegionLayout::new(
            self.region_max_data_pages,
            self.region_header_pages,
            self.page_size,
        );
        let trailing = if self.trailing_partial_region_pages > 0 {
            Some(RegionLayout::new(
                self.trailing_partial_region_pages,
                self.region_header_pages,
                self.page_size,
            ))
        } else {
            None
        };
        DatabaseLayout::new(self.full_regions, full_layout, trailing)
    }

    pub(super) fn set_layout(&mut self, layout: DatabaseLayout) {
        assert_eq!(
            self.layout().full_region_layout(),
            layout.full_region_layout()
        );
        if let Some(trailing) = layout.trailing_region_layout() {
            assert_eq!(trailing.get_header_pages(), self.region_header_pages);
            assert_eq!(trailing.page_size(), self.page_size);
            self.trailing_partial_region_pages = trailing.num_pages();
        } else {
            self.trailing_partial_region_pages = 0;
        }
        self.full_regions = layout.num_full_regions();
    }

    pub(super) fn primary_slot(&self) -> &TransactionHeader {
        &self.transaction_slots[self.primary_slot]
    }

    pub(super) fn secondary_slot(&self) -> &TransactionHeader {
        &self.transaction_slots[self.primary_slot ^ 1]
    }

    pub(super) fn secondary_slot_mut(&mut self) -> &mut TransactionHeader {
        &mut self.transaction_slots[self.primary_slot ^ 1]
    }

    pub(super) fn swap_primary_slot(&mut self) {
        self.primary_slot ^= 1;
    }

    // Figure out which slot to use as the primary when starting a repair. The repair process might
    // still switch to the other slot later, if the tree checksums turn out to be invalid.
    //
    // Returns true if we picked the original primary, or false if we swapped
    pub(super) fn pick_primary_for_repair(
        &mut self,
        repair_info: HeaderRepairInfo,
    ) -> Result<bool> {
        // If the primary was written using 2-phase commit, it's guaranteed to be valid. Don't look
        // at the secondary; even if it happens to have a valid checksum, Durability::Paranoid means
        // we can't trust it
        if self.two_phase_commit {
            if repair_info.primary_corrupted {
                return Err(StorageError::Corrupted(
                    "Primary is corrupted despite 2-phase commit".to_string(),
                ));
            }
            return Ok(true);
        }

        // Pick whichever slot is newer, assuming it has a valid checksum. This handles an edge case
        // where we crash during fsync(), and the only data that got written to disk was the god byte
        // update swapping the primary -- in that case, the primary contains a valid but out-of-date
        // transaction, so we need to load from the secondary instead
        if repair_info.primary_corrupted {
            if repair_info.secondary_corrupted {
                return Err(StorageError::Corrupted(
                    "Both commit slots are corrupted".to_string(),
                ));
            }
            self.swap_primary_slot();
            return Ok(false);
        }

        let secondary_newer =
            self.secondary_slot().transaction_id > self.primary_slot().transaction_id;
        if secondary_newer && !repair_info.secondary_corrupted {
            self.swap_primary_slot();
            return Ok(false);
        }

        Ok(true)
    }

    // TODO: consider returning an Err with the repair info
    pub(super) fn from_bytes(data: &[u8]) -> Result<(Self, HeaderRepairInfo), DatabaseError> {
        let invalid_magic_number = data[..MAGICNUMBER.len()] != MAGICNUMBER;

        let primary_slot = usize::from(data[GOD_BYTE_OFFSET] & PRIMARY_BIT != 0);
        let recovery_required = (data[GOD_BYTE_OFFSET] & RECOVERY_REQUIRED) != 0;
        let two_phase_commit = (data[GOD_BYTE_OFFSET] & TWO_PHASE_COMMIT) != 0;
        let page_size = get_u32(&data[PAGE_SIZE_OFFSET..]);
        let region_header_pages = get_u32(&data[REGION_HEADER_PAGES_OFFSET..]);
        let region_max_data_pages = get_u32(&data[REGION_MAX_DATA_PAGES_OFFSET..]);
        let full_regions = get_u32(&data[NUM_FULL_REGIONS_OFFSET..]);
        let trailing_data_pages = get_u32(&data[TRAILING_REGION_DATA_PAGES_OFFSET..]);
        let (slot0, slot0_corrupted) = TransactionHeader::from_bytes(
            &data[TRANSACTION_0_OFFSET..(TRANSACTION_0_OFFSET + TRANSACTION_SIZE)],
        )?;
        let (slot1, slot1_corrupted) = TransactionHeader::from_bytes(
            &data[TRANSACTION_1_OFFSET..(TRANSACTION_1_OFFSET + TRANSACTION_SIZE)],
        )?;
        let (primary_corrupted, secondary_corrupted) = if primary_slot == 0 {
            (slot0_corrupted, slot1_corrupted)
        } else {
            (slot1_corrupted, slot0_corrupted)
        };

        let result = Self {
            primary_slot,
            recovery_required,
            two_phase_commit,
            page_size,
            region_header_pages,
            region_max_data_pages,
            full_regions,
            trailing_partial_region_pages: trailing_data_pages,
            transaction_slots: [slot0, slot1],
        };
        let repair = HeaderRepairInfo {
            invalid_magic_number,
            primary_corrupted,
            secondary_corrupted,
        };
        Ok((result, repair))
    }

    pub(super) fn to_bytes(&self, include_magic_number: bool) -> [u8; DB_HEADER_SIZE] {
        let mut result = [0; DB_HEADER_SIZE];
        if include_magic_number {
            result[..MAGICNUMBER.len()].copy_from_slice(&MAGICNUMBER);
        }
        result[GOD_BYTE_OFFSET] = self.primary_slot.try_into().unwrap();
        if self.recovery_required {
            result[GOD_BYTE_OFFSET] |= RECOVERY_REQUIRED;
        }
        if self.two_phase_commit {
            result[GOD_BYTE_OFFSET] |= TWO_PHASE_COMMIT;
        }
        result[PAGE_SIZE_OFFSET..(PAGE_SIZE_OFFSET + size_of::<u32>())]
            .copy_from_slice(&self.page_size.to_le_bytes());
        result[REGION_HEADER_PAGES_OFFSET..(REGION_HEADER_PAGES_OFFSET + size_of::<u32>())]
            .copy_from_slice(&self.region_header_pages.to_le_bytes());
        result[REGION_MAX_DATA_PAGES_OFFSET..(REGION_MAX_DATA_PAGES_OFFSET + size_of::<u32>())]
            .copy_from_slice(&self.region_max_data_pages.to_le_bytes());
        result[NUM_FULL_REGIONS_OFFSET..(NUM_FULL_REGIONS_OFFSET + size_of::<u32>())]
            .copy_from_slice(&self.full_regions.to_le_bytes());
        result[TRAILING_REGION_DATA_PAGES_OFFSET
            ..(TRAILING_REGION_DATA_PAGES_OFFSET + size_of::<u32>())]
            .copy_from_slice(&self.trailing_partial_region_pages.to_le_bytes());
        let slot0 = self.transaction_slots[0].to_bytes();
        result[TRANSACTION_0_OFFSET..(TRANSACTION_0_OFFSET + slot0.len())].copy_from_slice(&slot0);
        let slot1 = self.transaction_slots[1].to_bytes();
        result[TRANSACTION_1_OFFSET..(TRANSACTION_1_OFFSET + slot1.len())].copy_from_slice(&slot1);

        result
    }
}

#[derive(Clone)]
pub(super) struct TransactionHeader {
    pub(super) version: u8,
    pub(super) user_root: Option<BtreeHeader>,
    pub(super) system_root: Option<BtreeHeader>,
    pub(super) transaction_id: TransactionId,
}

impl TransactionHeader {
    fn new(transaction_id: TransactionId) -> Self {
        Self {
            version: FILE_FORMAT_VERSION3,
            user_root: None,
            system_root: None,
            transaction_id,
        }
    }

    // Returned bool indicates whether the checksum was corrupted
    pub(super) fn from_bytes(data: &[u8]) -> Result<(Self, bool), DatabaseError> {
        let version = data[VERSION_OFFSET];
        match version {
            FILE_FORMAT_VERSION1 | FILE_FORMAT_VERSION2 => {
                return Err(DatabaseError::UpgradeRequired(version));
            }
            FILE_FORMAT_VERSION3 => {}
            _ => {
                return Err(StorageError::Corrupted(format!(
                    "Expected file format version <= {FILE_FORMAT_VERSION3}, found {version}",
                ))
                .into());
            }
        }
        let checksum = Checksum::from_le_bytes(
            data[SLOT_CHECKSUM_OFFSET..(SLOT_CHECKSUM_OFFSET + size_of::<Checksum>())]
                .try_into()
                .unwrap(),
        );
        let corrupted = checksum != xxh3_checksum(&data[..SLOT_CHECKSUM_OFFSET]);

        let user_root = if data[USER_ROOT_NON_NULL_OFFSET] != 0 {
            Some(BtreeHeader::from_le_bytes(
                data[USER_ROOT_OFFSET..(USER_ROOT_OFFSET + BtreeHeader::serialized_size())]
                    .try_into()
                    .unwrap(),
            ))
        } else {
            None
        };
        let system_root = if data[SYSTEM_ROOT_NON_NULL_OFFSET] != 0 {
            Some(BtreeHeader::from_le_bytes(
                data[SYSTEM_ROOT_OFFSET..(SYSTEM_ROOT_OFFSET + BtreeHeader::serialized_size())]
                    .try_into()
                    .unwrap(),
            ))
        } else {
            None
        };
        let transaction_id = TransactionId::new(get_u64(&data[TRANSACTION_ID_OFFSET..]));

        let result = Self {
            version,
            user_root,
            system_root,
            transaction_id,
        };

        Ok((result, corrupted))
    }

    pub(super) fn to_bytes(&self) -> [u8; TRANSACTION_SIZE] {
        assert_eq!(self.version, FILE_FORMAT_VERSION3);
        let mut result = [0; TRANSACTION_SIZE];
        result[VERSION_OFFSET] = self.version;
        if let Some(header) = self.user_root {
            result[USER_ROOT_NON_NULL_OFFSET] = 1;
            result[USER_ROOT_OFFSET..(USER_ROOT_OFFSET + BtreeHeader::serialized_size())]
                .copy_from_slice(&header.to_le_bytes());
        }
        if let Some(header) = self.system_root {
            result[SYSTEM_ROOT_NON_NULL_OFFSET] = 1;
            result[SYSTEM_ROOT_OFFSET..(SYSTEM_ROOT_OFFSET + BtreeHeader::serialized_size())]
                .copy_from_slice(&header.to_le_bytes());
        }
        result[TRANSACTION_ID_OFFSET..(TRANSACTION_ID_OFFSET + size_of::<u64>())]
            .copy_from_slice(&self.transaction_id.raw_id().to_le_bytes());
        let checksum = xxh3_checksum(&result[..SLOT_CHECKSUM_OFFSET]);
        result[SLOT_CHECKSUM_OFFSET..(SLOT_CHECKSUM_OFFSET + size_of::<Checksum>())]
            .copy_from_slice(&checksum.to_le_bytes());

        result
    }
}

#[cfg(test)]
mod test {
    use crate::backends::FileBackend;
    use crate::db::TableDefinition;
    use crate::tree_store::page_store::header::{
        GOD_BYTE_OFFSET, MAGICNUMBER, PRIMARY_BIT, RECOVERY_REQUIRED, TRANSACTION_0_OFFSET,
        TRANSACTION_1_OFFSET, TWO_PHASE_COMMIT, USER_ROOT_OFFSET,
    };
    use crate::{Database, DatabaseError, ReadableTable, StorageBackend};
    use crate::{ReadableDatabase, StorageError};
    use std::fs::OpenOptions;
    use std::io::{Error, ErrorKind, Read, Seek, SeekFrom, Write};
    use std::mem::size_of;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    const X: TableDefinition<&str, &str> = TableDefinition::new("x");

    #[derive(Debug)]
    struct FailingBackend {
        inner: FileBackend,
        fail: Arc<AtomicBool>,
    }

    impl FailingBackend {
        fn new(backend: FileBackend) -> Self {
            Self {
                inner: backend,
                fail: Arc::new(AtomicBool::new(false)),
            }
        }

        fn check_fail(&self) -> Result<(), std::io::Error> {
            if self.fail.load(Ordering::SeqCst) {
                return Err(std::io::Error::from(ErrorKind::Other));
            }

            Ok(())
        }
    }

    impl StorageBackend for FailingBackend {
        fn len(&self) -> Result<u64, std::io::Error> {
            self.check_fail()?;
            self.inner.len()
        }

        fn read(&self, offset: u64, out: &mut [u8]) -> Result<(), std::io::Error> {
            self.check_fail()?;
            self.inner.read(offset, out)
        }

        fn set_len(&self, len: u64) -> Result<(), std::io::Error> {
            self.check_fail()?;
            self.inner.set_len(len)
        }

        fn sync_data(&self) -> Result<(), std::io::Error> {
            self.check_fail()?;
            self.inner.sync_data()
        }

        fn write(&self, offset: u64, data: &[u8]) -> Result<(), std::io::Error> {
            self.check_fail()?;
            self.inner.write(offset, data)
        }

        fn close(&self) -> Result<(), Error> {
            self.inner.close()
        }
    }

    #[test]
    fn repair_allocator_checksums() {
        let tmpfile = crate::create_tempfile();
        let cloned = OpenOptions::new()
            .read(true)
            .write(true)
            .open(tmpfile.path())
            .unwrap();
        let backend = FailingBackend::new(FileBackend::new(cloned).unwrap());
        let fail = backend.fail.clone();
        let db = Database::builder().create_with_backend(backend).unwrap();
        let write_txn = db.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(X).unwrap();
            table.insert("hello", "world").unwrap();
        }
        write_txn.commit().unwrap();

        // Start a read to be sure the previous write isn't garbage collected
        let read_txn = db.begin_read().unwrap();

        let mut write_txn = db.begin_write().unwrap();
        {
            write_txn.set_quick_repair(true);
            let mut table = write_txn.open_table(X).unwrap();
            table.insert("hello", "world2").unwrap();
        }
        write_txn.commit().unwrap();
        drop(read_txn);
        // We want our commit to be the last commit in the database, so block the Database drop()
        // method from performing its own commit to trim the file
        fail.store(true, Ordering::SeqCst);
        drop(db);

        let mut file = tmpfile.as_file();

        file.seek(SeekFrom::Start(GOD_BYTE_OFFSET as u64)).unwrap();
        let mut buffer = [0u8; 1];
        file.read_exact(&mut buffer).unwrap();
        file.seek(SeekFrom::Start(GOD_BYTE_OFFSET as u64)).unwrap();
        buffer[0] |= RECOVERY_REQUIRED;
        buffer[0] &= !TWO_PHASE_COMMIT;
        file.write_all(&buffer).unwrap();

        // Overwrite the primary checksum to simulate a failure during commit
        let primary_slot_offset = if buffer[0] & PRIMARY_BIT == 0 {
            TRANSACTION_0_OFFSET
        } else {
            TRANSACTION_1_OFFSET
        };
        file.seek(SeekFrom::Start(
            (primary_slot_offset + USER_ROOT_OFFSET) as u64,
        ))
        .unwrap();
        file.write_all(&[0; size_of::<u128>()]).unwrap();

        #[allow(unused_mut)]
        let mut db2 = Database::create(tmpfile.path()).unwrap();
        let write_txn = db2.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(X).unwrap();
            assert_eq!(table.get("hello").unwrap().unwrap().value(), "world");
            table.insert("hello2", "world2").unwrap();
        }
        write_txn.commit().unwrap();

        // Locks are exclusive on Windows, so we can't concurrently overwrite the file
        #[cfg(not(target_os = "windows"))]
        {
            file.seek(SeekFrom::Start(GOD_BYTE_OFFSET as u64)).unwrap();
            let mut buffer = [0u8; 1];
            file.read_exact(&mut buffer).unwrap();

            // Overwrite the primary checksum to simulate a failure during commit
            let primary_slot_offset = if buffer[0] & PRIMARY_BIT == 0 {
                TRANSACTION_0_OFFSET
            } else {
                TRANSACTION_1_OFFSET
            };
            file.seek(SeekFrom::Start(
                (primary_slot_offset + USER_ROOT_OFFSET) as u64,
            ))
            .unwrap();
            file.write_all(&[0; size_of::<u128>()]).unwrap();

            assert!(!db2.check_integrity().unwrap());

            // Overwrite both checksums to simulate corruption
            file.seek(SeekFrom::Start(GOD_BYTE_OFFSET as u64)).unwrap();
            let mut buffer = [0u8; 1];
            file.read_exact(&mut buffer).unwrap();

            file.seek(SeekFrom::Start(
                (TRANSACTION_0_OFFSET + USER_ROOT_OFFSET) as u64,
            ))
            .unwrap();
            file.write_all(&[0; size_of::<u128>()]).unwrap();
            file.seek(SeekFrom::Start(
                (TRANSACTION_1_OFFSET + USER_ROOT_OFFSET) as u64,
            ))
            .unwrap();
            file.write_all(&[0; size_of::<u128>()]).unwrap();

            assert!(matches!(
                db2.check_integrity().unwrap_err(),
                DatabaseError::Storage(StorageError::Corrupted(_))
            ));
        }
    }

    #[test]
    fn repair_empty() {
        let tmpfile = crate::create_tempfile();
        let db = Database::builder().create(tmpfile.path()).unwrap();
        drop(db);

        let mut file = tmpfile.as_file();

        file.seek(SeekFrom::Start(GOD_BYTE_OFFSET as u64)).unwrap();
        let mut buffer = [0u8; 1];
        file.read_exact(&mut buffer).unwrap();
        file.seek(SeekFrom::Start(GOD_BYTE_OFFSET as u64)).unwrap();
        buffer[0] |= RECOVERY_REQUIRED;
        file.write_all(&buffer).unwrap();

        Database::open(tmpfile.path()).unwrap();
    }

    #[test]
    fn close_on_drop() {
        let tmpfile = crate::create_tempfile();
        let db = Database::builder()
            .set_cache_size(0)
            .create(tmpfile.path())
            .unwrap();
        let table_def: TableDefinition<u64, u64> = TableDefinition::new("x");
        let txn = db.begin_write().unwrap();
        {
            let mut table = txn.open_table(table_def).unwrap();
            table.insert(0, 0).unwrap();
        }
        txn.commit().unwrap();
        let txn = db.begin_read().unwrap();
        drop(db);
        assert!(matches!(
            txn.list_tables().err().unwrap(),
            StorageError::DatabaseClosed
        ));

        let mut file = tmpfile.as_file();

        file.seek(SeekFrom::Start(GOD_BYTE_OFFSET as u64)).unwrap();
        let mut buffer = [0u8; 1];
        file.read_exact(&mut buffer).unwrap();
        assert_eq!(buffer[0] & RECOVERY_REQUIRED, 0);
        drop(txn);
    }

    #[test]
    fn abort_repair() {
        let tmpfile = crate::create_tempfile();
        let db = Database::builder().create(tmpfile.path()).unwrap();
        drop(db);

        let mut file = tmpfile.as_file();

        file.seek(SeekFrom::Start(GOD_BYTE_OFFSET as u64)).unwrap();
        let mut buffer = [0u8; 1];
        file.read_exact(&mut buffer).unwrap();
        file.seek(SeekFrom::Start(GOD_BYTE_OFFSET as u64)).unwrap();
        buffer[0] |= RECOVERY_REQUIRED;
        buffer[0] &= !TWO_PHASE_COMMIT;
        file.write_all(&buffer).unwrap();

        let err = Database::builder()
            .set_repair_callback(|handle| handle.abort())
            .open(tmpfile.path())
            .unwrap_err();
        assert!(matches!(err, DatabaseError::RepairAborted));
    }

    #[test]
    fn repair_insert_reserve_regression() {
        let tmpfile = crate::create_tempfile();
        let db = Database::builder().create(tmpfile.path()).unwrap();

        let def: TableDefinition<&str, &[u8]> = TableDefinition::new("x");

        let write_txn = db.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(def).unwrap();
            let mut value = table.insert_reserve("hello", 5).unwrap();
            value.as_mut().copy_from_slice(b"world");
        }
        write_txn.commit().unwrap();

        let write_txn = db.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(def).unwrap();
            let mut value = table.insert_reserve("hello2", 5).unwrap();
            value.as_mut().copy_from_slice(b"world");
        }
        write_txn.commit().unwrap();

        drop(db);

        let mut file = tmpfile.as_file();

        file.seek(SeekFrom::Start(GOD_BYTE_OFFSET as u64)).unwrap();
        let mut buffer = [0u8; 1];
        file.read_exact(&mut buffer).unwrap();
        file.seek(SeekFrom::Start(GOD_BYTE_OFFSET as u64)).unwrap();
        buffer[0] |= RECOVERY_REQUIRED;
        file.write_all(&buffer).unwrap();

        Database::open(tmpfile.path()).unwrap();
    }

    #[test]
    fn magic_number() {
        // Test compliance with some, but not all, provisions recommended by
        // IETF Memo "Care and Feeding of Magic Numbers"

        // Test that magic number is not valid utf-8
        #[allow(invalid_from_utf8)]
        {
            assert!(std::str::from_utf8(&MAGICNUMBER).is_err());
        }
        // Test there is a octet with high-bit set
        assert!(MAGICNUMBER.iter().any(|x| *x & 0x80 != 0));
        // Test there is a non-printable ASCII character
        assert!(MAGICNUMBER.iter().any(|x| *x < 0x20 || *x > 0x7E));
        // Test there is a printable ASCII character
        assert!(MAGICNUMBER.iter().any(|x| *x >= 0x20 && *x <= 0x7E));
        // Test there is a printable ISO-8859 that's non-ASCII printable
        assert!(MAGICNUMBER.iter().any(|x| *x >= 0xA0));
        // Test there is a ISO-8859 control character other than 0x09, 0x0A, 0x0C, 0x0D
        assert!(MAGICNUMBER.iter().any(|x| *x < 0x09
            || *x == 0x0B
            || (0x0E <= *x && *x <= 0x1F)
            || (0x7F <= *x && *x <= 0x9F)));
    }
}
