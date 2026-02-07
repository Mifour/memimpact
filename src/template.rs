pub mod template_engine{
	use std::str::FromStr;
	use std::fmt::Write;

	pub fn format_memory_from_kib(value: u64) -> String{
		// every possible u64 values are handled, it is impossible to be stuck in an infinite loop
		const UNITS: [&str; 7] = ["KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB"];
	    let mut current = value;
	    let mut unit_index = 0;
	    while current >= 1024 && unit_index < UNITS.len() - 1 {
	        current >>= 10;
	        unit_index += 1;
	    }
	    format!("{}{}", current, UNITS[unit_index])
	}

	pub fn unescape(input: &str) -> Result<String, String> {
	    let mut out = String::with_capacity(input.len());
	    let mut chars = input.chars();
	
	    while let Some(c) = chars.next() {
	        if c == '\\' {
	            match chars.next() {
	                Some('n') => out.push('\n'),
	                Some('t') => out.push('\t'),
	                Some('\\') => out.push('\\'),
	                Some('"') => out.push('"'),
	                Some(other) => return Err(format!("Unknown escape: \\{}", other)),
	                None => return Err("Trailing backslash".into()),
	            }
	        } else {
	            out.push(c);
	        }
	    }
	    Ok(out)
	}

	
	pub struct MemorySample<'a> {
	    pub pid: i32,
	    pub process_name: &'a str,
	    pub current_bytes: u64,
	    pub max_bytes: u64,
	    pub timestamp: u64, // seconds since epoch
	}

	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub enum Field {
	    Pid,
	    ProcessName,
	    CurrentBytes,
	    MaxBytes,
	    CurrentHuman,
	    MaxHuman,
	    Timestamp,
	}

	impl FromStr for Field {
	
	    type Err = String;
	
	    fn from_str(input: &str) -> Result<Field, Self::Err> {
	        match input {
	            "Pid"  => Ok(Field::Pid),
	            "ProcessName"  => Ok(Field::ProcessName),
	            "CurrentBytes"  => Ok(Field::CurrentBytes),
	            "MaxBytes" => Ok(Field::MaxBytes),
	            "CurrentHuman" => Ok(Field::CurrentHuman),
	            "MaxHuman" => Ok(Field::MaxHuman),
	            "Timestamp" => Ok(Field::Timestamp),
	            _      => Err(format!("unknow field {:?}", input)),
	        }
	    }
	}

	#[derive(Debug)]
	pub struct Placeholder {
	    pub field: Field,
	}

	#[derive(Debug)]
	pub enum Token {
	    Literal(String),
	    Placeholder(Placeholder),
	}

	#[derive(Debug)]
	pub struct Template {
	    pub tokens: Vec<Token>,
	}
	
	impl Template {
	    pub fn parse(input: &str) -> Result<Self, String> {
	    	let mut tokens = Vec::new();
   	        let mut literal = String::new();
   	        let mut chars = input.chars().peekable();

   	        while let Some(c) = chars.next() {
   	            match c {
   	                '{' => {
   	                    if chars.peek() == Some(&'{') {
   	                        // Escaped literal "{"
   	                        chars.next();
   	                        literal.push('{');
   	                        continue;
   	                    }
   	
   	                    // Flush literal before placeholder
   	                    if !literal.is_empty() {
   	                        tokens.push(Token::Literal(std::mem::take(&mut literal)));
   	                    }
   	
   	                    // Read placeholder name
   	                    let mut name = String::new();
   	                    let mut closed = false;
   	                    
   	                    for next in chars.by_ref() {
   	                        if next == '}' {
   	                            closed = true;
   	                            break;
   	                        }
   	                        name.push(next);
   	                        //chars.next();
   	                    }

   	                    if !closed {
   	                        return Err("Unclosed placeholder".into());
   	                    }
   	
   	                    if name.is_empty() {
   	                        return Err("Empty placeholder {}".into());
   	                    }
   	
   	                    let field = Field::from_str(&name)?;
   	                    tokens.push(Token::Placeholder(Placeholder { field }));
   	                }

   	
   	                '}' => {
   	                    if chars.peek() == Some(&'}') {
   	                        // Escaped literal "}"
   	                        chars.next();
   	                        literal.push('}');
   	                    } else {
   	                        return Err("Unmatched '}'".into());
   	                    }
   	                }
   	
   	                _ => literal.push(c),
   	            }
   	        }
   	
   	        if !literal.is_empty() {
   	            tokens.push(Token::Literal(literal));
   	        }
   	
   	        Ok(Self { tokens })
		}

	    pub fn render(&self, sample: &MemorySample, out: &mut String) -> std::fmt::Result{
            for token in &self.tokens {
                match token {
                    Token::Literal(s) => out.push_str(s),
                    Token::Placeholder(placeholder) => {
                    	match placeholder.field {
	                        Field::Pid => write!(out, "{}", sample.pid)?,
	                        Field::ProcessName => out.push_str(sample.process_name),
	                        Field::CurrentBytes => write!(out, "{}", sample.current_bytes)?,
	                        Field::MaxBytes => write!(out, "{}", sample.max_bytes)?,
	                        Field::CurrentHuman => write!(out, "{}",format_memory_from_kib(sample.current_bytes))?,
	                        Field::MaxHuman => write!(out, "{}", format_memory_from_kib(sample.max_bytes))?,
	                        Field::Timestamp => write!(out, "{}", sample.timestamp)?,
	                    }
                    }
                }
            }
            Ok(())
        }
	}
}


/// tests

#[cfg(test)]
mod tests {
    use super::template_engine::*;

    fn sample() -> MemorySample<'static> {
        MemorySample {
            pid: 4242,
            process_name: "firefox",
            current_bytes: 10 * 1024 * 1024, // 10 MB
            max_bytes: 2 * 1024 * 1024 * 1024, // 2 GB
            timestamp: 1_700_000_000,
        }
    }

    // ---------------------------
    // format_memory
    // ---------------------------

    #[test]
    fn format_memory_basic_units() {
        assert_eq!(format_memory_from_kib(0), "0KiB");
        assert_eq!(format_memory_from_kib(1023), "1023KiB");
        assert_eq!(format_memory_from_kib(1024), "1MiB");
        assert_eq!(format_memory_from_kib(1024 * 1024), "1GiB");
    }

    #[test]
    fn format_memory_large_values() {
        assert_eq!(format_memory_from_kib(1024u64.pow(4)), "1PiB");
        assert_eq!(format_memory_from_kib(1024u64.pow(5)), "1EiB");
    }

    // ---------------------------
    // Field parsing
    // ---------------------------

    #[test]
    fn field_from_str_valid() {
        assert_eq!("Pid".parse::<Field>().unwrap(), Field::Pid);
        assert_eq!("ProcessName".parse::<Field>().unwrap(), Field::ProcessName);
        assert_eq!("CurrentBytes".parse::<Field>().unwrap(), Field::CurrentBytes);
        assert_eq!("MaxBytes".parse::<Field>().unwrap(), Field::MaxBytes);
        assert_eq!("CurrentHuman".parse::<Field>().unwrap(), Field::CurrentHuman);
        assert_eq!("MaxHuman".parse::<Field>().unwrap(), Field::MaxHuman);
        assert_eq!("Timestamp".parse::<Field>().unwrap(), Field::Timestamp);
    }

    #[test]
    fn field_from_str_invalid() {
        let err = "UnknownThing".parse::<Field>().unwrap_err();
        assert!(err.contains("unknow field"));
    }

    // ---------------------------
    // Template parsing
    // ---------------------------

    #[test]
    fn parse_literal_only() {
        let t = Template::parse("hello world").unwrap();
        assert_eq!(t.tokens.len(), 1);
        matches!(t.tokens[0], Token::Literal(_));
    }

    #[test]
    fn parse_single_placeholder() {
        let t = Template::parse("{Pid}").unwrap();
        assert_eq!(t.tokens.len(), 1);
        match &t.tokens[0] {
            Token::Placeholder(p) => assert_eq!(p.field, Field::Pid),
            _ => panic!("expected placeholder"),
        }
    }

    #[test]
    fn parse_mixed_tokens() {
        let t = Template::parse("PID={Pid} NAME={ProcessName}").unwrap();
        assert_eq!(t.tokens.len(), 4); // Lit, Ph, Lit, Ph
    }


    #[test]
    fn parse_unclosed_placeholder() {
        let err = Template::parse("hello {Pid").unwrap_err();
        assert_eq!(err, "Unclosed placeholder");
    }

    // ---------------------------
    // Rendering
    // ---------------------------

    #[test]
    fn render_simple_fields() {
        let t = Template::parse("PID={Pid} NAME={ProcessName}").unwrap();
        let mut out = String::new();
        t.render(&sample(), &mut out).unwrap();

        assert_eq!(out, "PID=4242 NAME=firefox");
    }

    #[test]
    fn render_byte_fields() {
        let t = Template::parse("{CurrentBytes}/{MaxBytes}").unwrap();
        let mut out = String::new();
        t.render(&sample(), &mut out).unwrap();

        assert_eq!(out, format!("{}/{}", sample().current_bytes, sample().max_bytes));
    }

    #[test]
    fn render_human_fields() {
        let t = Template::parse("{CurrentHuman} {MaxHuman}").unwrap();
        let mut out = String::new();
        t.render(&sample(), &mut out).unwrap();

        assert_eq!(out, "10GiB 2TiB"); 
        // NOTE: This reflects your bitshift logic, not real-world units.
    }

    #[test]
    fn render_timestamp_default_unix() {
        let t = Template::parse("{Timestamp}").unwrap();
        let mut out = String::new();
        t.render(&sample(), &mut out).unwrap();

        assert_eq!(out, sample().timestamp.to_string());
    }

    // ---------------------------
    // Edge behavior
    // ---------------------------

    #[test]
    fn render_multiple_same_placeholder() {
        let t = Template::parse("{Pid}-{Pid}-{Pid}").unwrap();
        let mut out = String::new();
        t.render(&sample(), &mut out).unwrap();

        assert_eq!(out, "4242-4242-4242");
    }

    #[test]
    fn render_adjacent_placeholders() {
        let t = Template::parse("{Pid}{ProcessName}").unwrap();
        let mut out = String::new();
        t.render(&sample(), &mut out).unwrap();

        assert_eq!(out, "4242firefox");
    }

    #[test]
    fn parse_escaped_open_brace() {
        let t = Template::parse("{{").unwrap();
        assert!(matches!(t.tokens[0], Token::Literal(ref s) if s == "{"));
    }
    
    #[test]
    fn parse_escaped_close_brace() {
        let t = Template::parse("}}").unwrap();
        assert!(matches!(t.tokens[0], Token::Literal(ref s) if s == "}"));
    }
    
    #[test]
    fn parse_literal_json() {
        let t = Template::parse(r#"{{"pid": {Pid}}}"#).unwrap();
        let mut out = String::new();
        t.render(&sample(), &mut out).unwrap();
        assert_eq!(out, r#"{"pid": 4242}"#);
    }

    #[test]
    fn error_if_placeholder_not_closed() {
        assert!(Template::parse("hello {Pid").is_err());
    }
    
    #[test]
    fn error_if_single_closing_brace() {
        assert!(Template::parse("hello } world").is_err());
    }
    
    #[test]
    fn error_if_empty_placeholder() {
        assert!(Template::parse("{}").is_err());
    }
    
}
