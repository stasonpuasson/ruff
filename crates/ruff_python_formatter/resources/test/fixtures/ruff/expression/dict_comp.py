{i: i for i in []}

{i: i for i in [1,]}

{
    a: a  # a
    for # for
    c  # c
    in  # in
    e  # e
}

{
    # above a
    a: a  # a
    # above for
    for # for
    # above c
    c  # c
    # above in
    in  # in
    # above e
    e  # e
    # above if
    if  # if
    # above f
    f  # f
    # above if2
    if  # if2
    # above g
    g  # g
}

{
    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa + bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb + [dddddddddddddddddd, eeeeeeeeeeeeeeeeeee]: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
    for
    ccccccccccccccccccccccccccccccccccccccc,
    ddddddddddddddddddd, [eeeeeeeeeeeeeeeeeeeeee, fffffffffffffffffffffffff]
    in
    eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeffffffffffffffffffffffffggggggggggggggggggggghhhhhhhhhhhhhhothermoreeand_even_moreddddddddddddddddddddd
    if
    fffffffffffffffffffffffffffffffffffffffffff < gggggggggggggggggggggggggggggggggggggggggggggg < hhhhhhhhhhhhhhhhhhhhhhhhhh
    if
    gggggggggggggggggggggggggggggggggggggggggggg
}

# Useful for tuple target (see https://github.com/astral-sh/ruff/issues/5779#issuecomment-1637614763)
{k: v for a, a, a, a, a, a, a, a, a, a, [a, a, a, a, a, a, a, a, a, a, a, a, a, a, a, a, a, a, a, a]  in this_is_a_very_long_variable_which_will_cause_a_trailing_comma_which_breaks_the_comprehension}
{k: v for a, a, a, a, a, a, a, a, a, a, (a, a, a, a, a, a, a, a, a, a, a, a, a, a, a, a, a, a, a, a,)  in this_is_a_very_long_variable_which_will_cause_a_trailing_comma_which_breaks_the_comprehension}

# Leading
{  # Leading
    k: v  # Trailing
    for a, a, a, a, a, a, a, a, a, a, (  # Trailing
        a,
        a,
        a,
        a,
        a,
        a,
        a,
        a,
        a,  # Trailing
        a,
        a,
        a,
        a,
        a,
        a,
        a,
        a,
        a,
        a,
        a,  # Trailing
    ) in this_is_a_very_long_variable_which_will_cause_a_trailing_comma_which_breaks_the_comprehension  # Trailing
}  # Trailing
# Trailing