use repulse::{Twitch, helix};

fn main() {
    let twi = Twitch::new();
    
    let a = helix::search_channel(&twi, "bigbass__");
    let b = helix::search_channel(&twi, "twitchplayspokemon");
    
    //let a = helix::get_streams(&mut twi, vec!["jp_xinnam", "twitchplayspokemon"]);
    //let a = helix::search_category(&mut twi, "Old School RuneScape");
    //let b = helix::get_streams_by_games(&mut twi, vec![&a.id]);
    
    
    
    //std::fs::write("/tmp/helixoutput.txt", format!("{:#?}", b)).unwrap();
    println!("{:#?}", a);
    println!("{:#?}", b);
    //println!("LENGTH: {:?}", b.len());
}