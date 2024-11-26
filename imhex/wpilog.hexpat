#pragma endianness little
#pragma base_address 0x00

#include <std/mem>
#include <std/core>

import type.magic as magic;

#define SUPPORTED_VERSION 0x0100

// Functions

fn vu32(auto data) {
    const auto len = std::core::member_count(data);

    u32 o = u32(data[0]);
    for (u8 i = 1, i < len, i = i + 1) {
        o = o | (u32(data[i]) << (8 * i));
    }

    return o;
};

fn vu64(auto data) {
    const auto len = std::core::member_count(data);

    u64 o = u64(data[0]);
    for (u8 i = 1, i < len, i = i + 1) {
        o = o | (u64(data[i]) << (8 * i));
    }

    return o;
};

// Data

struct Header {
    magic::Magic<"WPILOG"> magic;
    
    u16 version;
    std::assert_warn(version == SUPPORTED_VERSION, "Unsupported .wpilog version!");
    
    std::string::SizedString<u32> metadata;
};

bitfield Lengths {
    entry_id_length: 2;
    payload_size_length: 2;
    timestamp_length: 3;
    padding: 1;
};

enum ControlRecordType : u8 {
    Start = 0x00,
    Finish = 0x01,
    Metadata = 0x02
};

struct Record {
    Lengths lengths;

    u8 entry_id[lengths.entry_id_length + 1] [[format("vu32")]];
    u8 payload_size[lengths.payload_size_length + 1] [[format("vu32")]];
    u8 timestamp[lengths.timestamp_length + 1] [[format("vu64")]];
    
    if (vu32(entry_id) == 0) {
        ControlRecordType cr_type;
        u32 target_entry_id;

        match (cr_type) {
            (ControlRecordType::Start): {
                std::string::SizedString<u32> entry_name;
                std::string::SizedString<u32> entry_type;
                std::string::SizedString<u32> metadata;
            }
            (ControlRecordType::Metadata): {
                std::string::SizedString<u32> metadata;
            }
        }
    } else {
        u8 payload[vu32(payload_size)];
    }
};

Header header @ $;
Record records[while(!std::mem::eof())] @ $;